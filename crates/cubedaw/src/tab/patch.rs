use std::iter;

use anyhow::Result;

use cubedaw_command::{node::NodeStateUpdate, patch::CableAddOrRemove};
use cubedaw_lib::{Cable, CableConnection, CableTag, Id, IdMap, NodeData, NodeEntry, Track};
use egui::{emath::TSTransform, pos2, Pos2, Rangef, Rect, Vec2};
use resourcekey::ResourceKey;
use unwrap_todo::UnwrapTodo;

use crate::{
    command::node::{UiNodeAddOrRemove, UiNodeMove, UiNodeSelect},
    context::UiStateTracker,
    state::{ephemeral::NodeEphemeralState, ui::NodeUiState},
    widget::DragValue,
};

pub struct PatchTab {
    id: Id<crate::app::Tab>,

    track_id: Option<Id<Track>>,

    transform: TSTransform,

    // used for when the user has clicked on a node to add but hasn't placed it yet; the node doesn't "exist" yet.
    currently_held_node: Option<NodeData>,
    // used for when the user draws a cable. duh.
    currently_drawn_cable: Option<CurrentlyDrawnCable>,
}
#[derive(Clone)]
struct CurrentlyDrawnCable {
    /// Id of the cable. This is usually `Id::arbitrary()`, but in some circumstances is the id of a previously existing cable.
    pub id: Id<Cable>,

    /// The node slot that this cable is attached to. The other side is the mouse cursor.
    pub attached: NodeSlotDescriptor,

    /// The input node slot that this was originally attached to, or `None` if this was a just-created cable. Used when the user drags the end of an existing cable.
    pub originally_attached: Option<(NodeSlotDescriptor, CableConnection)>,

    /// If the user drags the end of this cable over another cable, the original cable is replaced. This holds the original cable in case the user moves their cursor away from the slot, making the original cable appear again.
    pub cable_that_this_replaces: Option<(Id<Cable>, Cable)>,

    /// Cable tag of this cable. Usually this is `CableTag::Disconnected`, but can be `CableTag::Valid` the frame before a cable is added.
    pub tag: CableTag,
}

fn transform_viewport(transform: TSTransform, viewport: Rect) -> TSTransform {
    TSTransform::new(
        transform.translation + viewport.center().to_vec2(),
        transform.scaling,
    )
}

impl crate::Screen for PatchTab {
    fn create(_state: &cubedaw_lib::State, ui_state: &crate::UiState) -> Self
    where
        Self: Sized,
    {
        Self {
            id: Id::arbitrary(),

            track_id: ui_state.get_single_selected_track(),

            transform: TSTransform::IDENTITY,

            currently_held_node: None,
            currently_drawn_cable: None,
        }
    }

    fn id(&self) -> Id<crate::app::Tab> {
        self.id
    }

    fn title(&self) -> egui::WidgetText {
        "Patch Tab".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) -> Result<()> {
        egui::CentralPanel::default()
            .show_inside(ui, |ui| -> Result<()> {
                if self.track_id.is_some() {
                    let parent_layer_id = ui.layer_id();
                    let screen_viewport = ui.max_rect();
                    let transform = transform_viewport(self.transform, screen_viewport);
                    // we use an area here because it's the only way to render something with custom transforms above another layer.
                    // kinda jank (and there doesn't seem to be a way to delete an area), but oh well.
                    egui::Area::new(parent_layer_id.id.with((parent_layer_id.order, self.id)))
                        .movable(false)
                        .constrain_to(screen_viewport)
                        .order(parent_layer_id.order)
                        .show(ui.ctx(), |ui| -> Result<()> {
                            // ui.push_id(id_source, add_contents)
                            let layer_id = ui.layer_id();

                            let viewport = transform.inverse() * screen_viewport;
                            ui.set_clip_rect(viewport);
                            let viewport_interaction = ui.interact(
                                viewport,
                                layer_id.id.with("patch_move"),
                                egui::Sense::click_and_drag(),
                            );
                            if viewport_interaction.contains_pointer() {
                                let (scroll_delta, zoom) =
                                    ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
                                if scroll_delta != Vec2::ZERO {
                                    self.transform.translation += scroll_delta;
                                }
                                if zoom != 1.0 {
                                    if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                                        let hover_pos = transform.inverse() * hover_pos;

                                        // the zoom center should stay at the same location after the transform
                                        // pos * scale + t = pos * (scale * zoom) + new_t
                                        // new_t = pos * scale + t - pos * scale * zoom
                                        // new_t = t + pos * scale * (1 - zoom)
                                        self.transform.translation += hover_pos.to_vec2()
                                            * self.transform.scaling
                                            * (1.0 - zoom);
                                    }
                                    self.transform.scaling *= zoom;
                                }
                            }
                            let transform = transform_viewport(self.transform, screen_viewport);
                            let viewport = transform.inverse() * screen_viewport;

                            ui.ctx().set_transform_layer(layer_id, transform);
                            ui.ctx().set_sublayer(parent_layer_id, layer_id);

                            ui.set_clip_rect(viewport);
                            self.node_view(ctx, ui, viewport, viewport_interaction)?;

                            Ok(())
                        })
                        .inner?;
                } else {
                    ui.with_layout(
                        egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                        |ui| {
                            ui.label("No track selected");
                        },
                    );
                }
                Ok(())
            })
            .inner?;
        Ok(())
    }

    fn drop(self: Box<Self>, _egui_ctx: &egui::Context) {
        // TODO how do we delete an area from egui memory
        // egui_ctx.memory_mut(|m| m.areas_mut().)
    }
}

impl PatchTab {
    fn node_view(
        &mut self,
        ctx: &mut crate::Context,
        ui: &mut egui::Ui,
        viewport: Rect,
        viewport_interaction: egui::Response,
    ) -> Result<()> {
        // setup
        let Some(track_id) = self.track_id else {
            unreachable!()
        };
        let Track { patch, .. } = ctx.state.tracks.force_get(track_id);
        let track_ui = ctx.ui_state.tracks.force_get(track_id);
        let track_ephemeral = ctx.ephemeral_state.tracks.force_get_mut(track_id);

        let patch_ui = &ctx
            .ui_state
            .tracks
            .get(track_id)
            .expect("track state exists but not ui state?")
            .patch;

        ui.set_clip_rect(viewport);
        let painter = ui.painter();

        let (primary_clicked, secondary_clicked, screen_hover_pos) = ui.input(|i| {
            (
                i.pointer.primary_clicked(),
                i.pointer.secondary_clicked(),
                i.pointer.hover_pos(),
            )
        });
        let pointer_pos = if viewport_interaction.contains_pointer() {
            screen_hover_pos.map(|mut pos| {
                if let Some(transform) = ui
                    .ctx()
                    .memory(|m| m.layer_transforms.get(&ui.layer_id()).copied())
                {
                    pos = transform.inverse() * pos;
                }
                pos
            })
        } else {
            None
        };

        // background
        painter.rect_filled(
            viewport,
            egui::Rounding::ZERO,
            ui.visuals().extreme_bg_color,
        );
        const DOT_SPACING: f32 = 16.0;
        const DOT_RADIUS: f32 = 1.5;

        // don't draw too many dots
        if viewport.width().max(viewport.height()) < 960.0 {
            for x in (viewport.left() / DOT_SPACING - DOT_RADIUS).ceil() as i32.. {
                if x as f32 * DOT_SPACING > viewport.right() + DOT_RADIUS {
                    break;
                }
                for y in (viewport.top() / DOT_SPACING - DOT_RADIUS).ceil() as i32.. {
                    if y as f32 * DOT_SPACING > viewport.bottom() + DOT_RADIUS {
                        break;
                    }

                    painter.circle_filled(
                        pos2(x as f32 * DOT_SPACING, y as f32 * DOT_SPACING),
                        DOT_RADIUS,
                        ui.visuals().faint_bg_color,
                    );
                }
            }
        }

        // TODO: this is a WIP "add node" menu. later we would want a search bar and whatnot.
        viewport_interaction.context_menu(|ui| {
            ui.menu_button("Add...", |ui| {
                let mut node_added: Option<ResourceKey> = None;
                // TODO make this a search bar
                if ui.button("Math").clicked() {
                    node_added = Some(resourcekey::literal!("cubedaw:math"));
                }
                if ui.button("Oscillator").clicked() {
                    node_added = Some(resourcekey::literal!("cubedaw:oscillator"));
                }
                if ui.button("Note Output").clicked() {
                    node_added = Some(resourcekey::literal!("builtin:note_output"));
                }
                if ui.button("Track Output").clicked() {
                    node_added = Some(resourcekey::literal!("builtin:track_output"));
                }

                if let Some(key) = node_added {
                    ui.close_menu();

                    let entry = ctx.node_registry.get(&key).expect("wut");
                    self.currently_held_node = Some(NodeData::new_disconnected(
                        key,
                        entry
                            .node_thingy
                            .create(&crate::node::NodeCreationContext::default()),
                    ));
                }
            });
        });

        // cables are rendered below everything else; save a ShpaeIdx for them!
        let cable_shapeidx = ui.painter().add(egui::Shape::Noop);

        // nodes; their interactions, rendering, whew
        let mut hovered_node_slot = None;
        let mut cable_drag_stopped = false;

        let result = ctx
            .ephemeral_state
            .drag
            .handle(Id::new("nodes"), |prepared| -> Result<_> {
                let mut node_results: IdMap<NodeEntry, CubedawNodeUiContextResult> = IdMap::new();

                // nodes
                if viewport_interaction.secondary_clicked() {
                    self.currently_held_node = None;
                }

                let currently_held_node_is_some = self.currently_held_node.is_some();

                let mut handle_node =
                    |prepared: &mut crate::util::Prepared<(Id<Track>, Id<NodeEntry>)>,
                     ui: &mut egui::Ui,
                     node_data: &NodeEntry,
                     node_id: Option<Id<NodeEntry>>,
                     node_ui: &NodeUiState,
                     tracker: &mut UiStateTracker|
                     -> Result<CubedawNodeUiContextResult> {
                        // Some(node_id, ephemeral_state) if node actually exists, None if the node is just there for rendering
                        // (e.g. the user is adding a node and is choosing where to place it)
                        let mut real_node_data = node_id.map(|node_id| {
                            (
                                node_id,
                                track_ephemeral.nodes.get_mut_or_insert_default(node_id),
                            )
                        });

                        let pos = if node_ui.selected
                            && let Some(offset) = prepared.movement()
                        {
                            node_ui.pos + offset
                        } else {
                            node_ui.pos
                        };

                        let node_max_rect = Rect::from_x_y_ranges(
                            pos.x..=pos.x + node_ui.width,
                            pos.y..=match real_node_data {
                                Some((_, ref node_ephemeral)) => pos.y + node_ephemeral.size.y,
                                None => {
                                    // would be f32::INFINITY but egui needs a finite rect
                                    viewport.bottom() + 128.0
                                }
                            },
                        );

                        let mut frame_ui = ui.child_ui_with_id_source(
                            node_max_rect,
                            egui::Layout::top_down(egui::Align::Min),
                            node_id.unwrap_or(Id::new("currently_held_node")),
                            None,
                        );
                        if currently_held_node_is_some {
                            // TODO make ui uninteractable without setting fade out color
                            // frame_ui.disable()
                        }
                        if node_id.is_some()
                            && !node_max_rect.intersects(viewport)
                            && prepared.dragged_thing() != node_id.map(Id::cast)
                        {
                            // node isn't visible, hide it
                            frame_ui.set_invisible();
                        }
                        frame_ui.spacing_mut().item_spacing = egui::vec2(8.0, 4.0);

                        let mut frame = egui::Frame::window(ui.style()).inner_margin(8.0);
                        if node_ui.selected {
                            // TODO actually implement selection colors/strokes
                            frame.stroke = egui::Stroke::new(
                                frame.stroke.width * 1.2,
                                egui::Color32::from_gray(96),
                            );
                            frame.fill = egui::Color32::from_gray(32);
                        }

                        let mut default_node_ephemeral = NodeEphemeralState::default();

                        let mut ui_ctx = CubedawNodeUiContext::new(
                            node_id,
                            track_id,
                            node_data,
                            match real_node_data {
                                Some((_, ref mut node_ephemeral)) => node_ephemeral,
                                None => &mut default_node_ephemeral,
                            },
                            self.currently_drawn_cable.clone(),
                        );

                        let frame_rect: Rect;

                        // node frame
                        let mut frame_prepared = frame.begin(&mut frame_ui);
                        frame_prepared
                            .content_ui
                            .style_mut()
                            .interaction
                            .selectable_labels = false;
                        {
                            if let Some(node_id) = node_id {
                                let drag_response = frame_ui.allocate_rect(
                                    Rect::from_min_size(node_ui.pos, ui_ctx.node_ephemeral.size),
                                    egui::Sense::click_and_drag(),
                                );
                                prepared.process_interaction(
                                    node_id.cast(),
                                    &drag_response,
                                    (track_id, node_id),
                                    node_ui.selected,
                                );
                            }
                            let node_state = node_data.data.inner.as_ref();
                            let mut node_state_copy: Box<[u8]> = node_state.into();

                            // TODO add header colors
                            let node_thingy = ctx
                                .node_registry
                                .get(&node_data.data.key)
                                .todo()
                                .node_thingy
                                .as_ref();

                            frame_prepared
                                .content_ui
                                .label(node_thingy.title(node_state)?);
                            frame_prepared.content_ui.separator();
                            node_thingy.ui(
                                &mut node_state_copy,
                                &mut frame_prepared.content_ui,
                                &mut ui_ctx,
                            )?;

                            if *node_state_copy != *node_state
                                && let Some(node_id) = node_id
                            {
                                tracker.add(NodeStateUpdate::new(
                                    node_id,
                                    track_id,
                                    node_state_copy,
                                    ui_ctx.inputs.iter().map(|i| i.value).collect(),
                                    node_data.inputs().iter().map(|i| i.bias).collect(),
                                    ui_ctx.outputs.len() as u32,
                                    node_data.outputs().len() as u32,
                                ));
                            }

                            frame_rect =
                                frame_prepared.content_ui.min_rect() + frame.total_margin();
                            frame_prepared.allocate_space(&mut frame_ui);
                            ui_ctx.node_ephemeral.size = frame_rect.size();
                        }
                        frame_prepared.paint(&frame_ui);

                        // node inputs
                        //
                        // index: either input index or output index, depending on is_output
                        // y_pos: screen y pos
                        // cable_index: for inputs, either Some(the 0-based index of the cable this is connected to) or None for not being connected to a cable. for outputs, unused.
                        // is_output: duh
                        for ((index, (y_pos, cable_index)), is_output) in ui_ctx
                            .inputs
                            .iter()
                            .enumerate()
                            .flat_map(|(idx, input)| {
                                iter::once((idx, (input.y_pos, None))).chain(
                                    input.cables.iter().enumerate().map(
                                        move |(cable_idx, cable_input)| {
                                            (idx, (cable_input.y_pos, Some(cable_idx as u32)))
                                        },
                                    ),
                                )
                            })
                            .zip(iter::repeat(false))
                            .chain(
                                ui_ctx
                                    .outputs
                                    .iter()
                                    .map(|o| (o.y_pos, None))
                                    .enumerate()
                                    .zip(iter::repeat(true)),
                            )
                        {
                            let pos = egui::pos2(
                                if is_output {
                                    node_max_rect.right()
                                } else {
                                    node_max_rect.left()
                                },
                                y_pos,
                            );

                            // TODO add configurable styles for this
                            let slot_radius = 4.0;

                            let mut hovered = false;
                            if let Some(node_id) = node_id {
                                let response = frame_ui
                                    .allocate_rect(
                                        Rect::from_min_size(pos, Vec2::ZERO).expand(
                                            slot_radius + 4.0 + ui.input(|i| i.aim_radius()),
                                        ),
                                        egui::Sense::drag(),
                                    )
                                    .on_hover_cursor(egui::CursorIcon::PointingHand);

                                let slot_descriptor = if is_output {
                                    NodeSlotDescriptor::Output {
                                        node: node_id,
                                        output_index: index as u32,
                                    }
                                } else {
                                    NodeSlotDescriptor::Input {
                                        node: node_id,
                                        input_index: index as u32,
                                        conn_index: cable_index,
                                    }
                                };

                                if response.drag_started() {
                                    if let Some(cable_index) = cable_index
                                        && let Some(conn) = node_data
                                            .inputs()
                                            .get(index)
                                            .expect("unreachable")
                                            .connections
                                            .get(cable_index as usize)
                                    {
                                        // if the slot is an input and there already is a cable there, take control of it
                                        let cable = patch.cable(conn.id).expect("unreachable");

                                        self.currently_drawn_cable = Some(CurrentlyDrawnCable {
                                            id: conn.id,
                                            attached: NodeSlotDescriptor::Output {
                                                node: cable.input_node,
                                                output_index: cable.input_output_index,
                                            },
                                            originally_attached: Some((
                                                slot_descriptor,
                                                conn.clone(),
                                            )),
                                            cable_that_this_replaces: None,
                                            tag: CableTag::Valid,
                                        });
                                        tracker
                                            .add_weak(CableAddOrRemove::removal(conn.id, track_id));
                                    } else {
                                        // create a new cable
                                        self.currently_drawn_cable = Some(CurrentlyDrawnCable {
                                            id: Id::arbitrary(),
                                            attached: slot_descriptor,
                                            originally_attached: None,

                                            cable_that_this_replaces: None,
                                            tag: CableTag::Disconnected,
                                        });
                                    }

                                    tracker.make_next_command_strong();
                                } else if response.drag_stopped() {
                                    cable_drag_stopped = true;
                                }
                                if response.contains_pointer() {
                                    hovered = true;
                                    hovered_node_slot = Some(slot_descriptor);
                                }
                            }
                            let visuals = if hovered {
                                frame_ui.visuals().widgets.hovered
                            } else {
                                frame_ui.visuals().widgets.noninteractive
                            };
                            let slot_fill = visuals.bg_fill;
                            let slot_stroke = visuals.bg_stroke;

                            frame_ui
                                .painter()
                                .circle(pos, slot_radius, slot_fill, slot_stroke);
                        }

                        ui_ctx.apply(tracker);

                        Ok(ui_ctx.finish(frame_rect))
                    };
                for (node_id, node_data) in patch.nodes() {
                    let node_ui = patch_ui.nodes.get(node_id).expect("nonexistent node ui");

                    let result = handle_node(
                        prepared,
                        ui,
                        node_data,
                        Some(node_id),
                        node_ui,
                        &mut ctx.tracker,
                    )?;

                    node_results.insert(node_id, result);
                }

                if let Some(node_data) = self.currently_held_node.take() {
                    if let Some(pointer_pos) = pointer_pos {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::AllScroll);
                        let fake_entry = NodeEntry::new(node_data, 0, 0);
                        let result = handle_node(
                            prepared,
                            ui,
                            &fake_entry,
                            None,
                            &NodeUiState {
                                selected: true,
                                pos: pointer_pos,
                                width: 128.0,
                            },
                            &mut ctx.tracker,
                        )?;
                        let node_data = fake_entry.data;
                        if primary_clicked {
                            // place the node
                            ctx.tracker.add(UiNodeAddOrRemove::addition(
                                Id::arbitrary(),
                                node_data,
                                result.inputs.into_iter().map(|input| input.value).collect(),
                                result.outputs.len() as u32,
                                track_id,
                                NodeUiState {
                                    selected: true,
                                    pos: pointer_pos,
                                    width: 128.0, // TODO impl node widths
                                },
                            ))
                        } else if !secondary_clicked {
                            self.currently_held_node = Some(node_data);
                        }
                    } else {
                        self.currently_held_node = Some(node_data);
                    }
                }

                Ok(node_results)
            });
        {
            let should_deselect_everything =
                result.should_deselect_everything || viewport_interaction.clicked();
            let selection_changes = result.selection_changes;
            if should_deselect_everything {
                // TODO rename these
                for (&node_id2, node_ui) in &track_ui.patch.nodes {
                    if node_ui.selected
                        && !matches!(selection_changes.get(&(track_id, node_id2)), Some(true))
                    {
                        ctx.tracker
                            .add(UiNodeSelect::new(track_id, node_id2, false));
                    }
                }
                for (&(track_id, node_id), &selected) in &selection_changes {
                    if selected
                        && !ctx
                            .ui_state
                            .tracks
                            .get(track_id)
                            .and_then(|t| t.patch.nodes.get(node_id))
                            .is_some_and(|n| n.selected)
                    {
                        ctx.tracker.add(UiNodeSelect::new(track_id, node_id, true));
                    }
                }
            } else {
                for (&(track_id, node_id), &selected) in &selection_changes {
                    ctx.tracker
                        .add(UiNodeSelect::new(track_id, node_id, selected));
                }
            }
            if let Some(finished_drag_offset) = result.movement {
                for (&node_id, node_ui) in &track_ui.patch.nodes {
                    if node_ui.selected {
                        ctx.tracker
                            .add(UiNodeMove::new(node_id, track_id, finished_drag_offset));
                    }
                }
            }
        }

        let node_results: IdMap<NodeEntry, CubedawNodeUiContextResult> = result.inner?;

        // cables

        let mut cable_shapes = Vec::new();
        let mut draw_cable = |output_pos: Pos2, input_pos: Pos2, tag: CableTag| {
            // TODO make this configurable
            let cable_stroke = egui::Stroke::new(
                4.0,
                match tag {
                    CableTag::Invalid => ui.visuals().error_fg_color,
                    CableTag::Valid => egui::Color32::from_gray(128),
                    CableTag::Disconnected => egui::Color32::from_gray(100),
                },
            );

            if !viewport.intersects(Rect::from_points(&[output_pos, input_pos])) {
                return;
            }

            let mut control_point_distance = (input_pos.x - output_pos.x).abs() * 0.5;
            const MIN_BEZIER_DISTANCE: f32 = 70.0;
            if control_point_distance.abs() < MIN_BEZIER_DISTANCE {
                control_point_distance = MIN_BEZIER_DISTANCE.copysign(control_point_distance);
            }

            cable_shapes.push(
                egui::epaint::CubicBezierShape {
                    points: [
                        output_pos,
                        output_pos + Vec2::new(control_point_distance, 0.0),
                        input_pos - Vec2::new(control_point_distance, 0.0),
                        input_pos,
                    ],
                    closed: false,
                    fill: egui::Color32::TRANSPARENT,
                    stroke: cable_stroke.into(),
                }
                .into(),
            );
        };

        if let Some(pointer_pos) = pointer_pos {
            if let Some(mut currently_drawn_cable) = self.currently_drawn_cable.take() {
                let viable_cable = match (hovered_node_slot, currently_drawn_cable.attached) {
                    (
                        Some(NodeSlotDescriptor::Output {
                            node: input_node,
                            output_index,
                        }),
                        NodeSlotDescriptor::Input {
                            node: output_node,
                            input_index,
                            conn_index: cable_index,
                        },
                    )
                    | (
                        Some(NodeSlotDescriptor::Input {
                            node: output_node,
                            input_index,
                            conn_index: cable_index,
                        }),
                        NodeSlotDescriptor::Output {
                            node: input_node,
                            output_index,
                        },
                    ) => Some((
                        Cable {
                            input_node,
                            input_output_index: output_index,

                            output_node,
                            output_input_index: input_index,
                            output_cable_index: cable_index.unwrap_or(0),

                            tag: CableTag::Invalid,
                        },
                        cable_index,
                    )),
                    _ => None,
                };
                let currently_drawn_cable_exists_in_patch =
                    patch.cable(currently_drawn_cable.id).is_some();

                let mut should_render_currently_drawn_cable = true; // false if a "real" cable already replaced this one
                if let Some((cable, cable_index)) = viable_cable {
                    if currently_drawn_cable_exists_in_patch {
                        should_render_currently_drawn_cable = false;
                    } else {
                        if let Some(cable_index) = cable_index
                            && let Some(conn) = cable
                                .node_input(patch)
                                .connections
                                .get(cable_index as usize)

                            // don't delete the cable at this index if it's where the cable is originally attached!
                            && currently_drawn_cable.originally_attached.as_ref().is_none_or(
                                |(descriptor, _)| {
                                    descriptor
                                        != &NodeSlotDescriptor::Input {
                                            node: cable.output_node,
                                            input_index: cable.output_input_index,
                                            conn_index: Some(cable_index),
                                        }
                                },
                            )
                        {
                            currently_drawn_cable.cable_that_this_replaces =
                                Some((conn.id, patch.cable(conn.id).todo().clone()));
                            ctx.tracker
                                .add_weak(CableAddOrRemove::removal(conn.id, track_id));
                        }
                        ctx.tracker.add_weak(CableAddOrRemove::addition(
                            currently_drawn_cable.id,
                            cable,
                            track_id,
                        ));
                    }
                } else {
                    currently_drawn_cable.tag = CableTag::Disconnected;

                    if currently_drawn_cable_exists_in_patch {
                        ctx.tracker.add_weak(CableAddOrRemove::removal(
                            currently_drawn_cable.id,
                            track_id,
                        ));
                        if let Some((cable_id, cable)) =
                            currently_drawn_cable.cable_that_this_replaces.take()
                        {
                            ctx.tracker
                                .add_weak(CableAddOrRemove::addition(cable_id, cable, track_id));
                        }
                    }
                }
                if should_render_currently_drawn_cable {
                    let attached_pos = currently_drawn_cable.attached.get_pos(&node_results);
                    match currently_drawn_cable.attached {
                        NodeSlotDescriptor::Input { .. } => {
                            let output_pos = match hovered_node_slot {
                                Some(slot @ NodeSlotDescriptor::Output { .. }) => {
                                    slot.get_pos_raw(&node_results)
                                }
                                _ => pointer_pos,
                            };
                            draw_cable(output_pos, attached_pos, currently_drawn_cable.tag)
                        }
                        NodeSlotDescriptor::Output { .. } => {
                            let input_pos = match hovered_node_slot {
                                Some(slot @ NodeSlotDescriptor::Input { .. }) => {
                                    slot.get_pos_raw(&node_results)
                                }
                                _ => pointer_pos,
                            };
                            draw_cable(attached_pos, input_pos, currently_drawn_cable.tag);
                        }
                    }
                }
                if cable_drag_stopped {
                    // did it actually do anything? no? guess all those commands were for nothing then. delete the commands
                    let was_added = currently_drawn_cable.originally_attached.is_none()
                        && currently_drawn_cable_exists_in_patch;
                    let was_deleted = currently_drawn_cable.originally_attached.is_some()
                        && !currently_drawn_cable_exists_in_patch;
                    let was_moved = currently_drawn_cable
                        .originally_attached
                        .as_ref()
                        .is_some_and(|(node_slot, _)| Some(*node_slot) != hovered_node_slot);
                    if !(was_added || was_deleted || was_moved) {
                        ctx.tracker.delete_last_command();
                    }
                } else {
                    self.currently_drawn_cable = Some(currently_drawn_cable);
                }
            }
        }

        for (_cable_id, cable) in patch.cables() {
            draw_cable(
                node_results
                    .force_get(cable.input_node)
                    .get_output_pos(cable.input_output_index),
                node_results
                    .force_get(cable.output_node)
                    .get_input_pos(cable.output_input_index, Some(cable.output_cable_index)),
                cable.tag,
            );
        }

        ui.painter()
            .set(cable_shapeidx, egui::Shape::Vec(cable_shapes));

        Ok(())
    }

    pub fn select_track(&mut self, track_id: Option<Id<Track>>) {
        self.track_id = track_id;
    }
}

struct CubedawNodeUiContext<'a> {
    node_id: Option<Id<NodeEntry>>,
    track_id: Id<Track>,
    node_data: &'a NodeEntry,

    node_ephemeral: &'a mut NodeEphemeralState,
    inputs: Vec<CubedawNodeUiContextInputData>,
    outputs: Vec<CubedawNodeUiContextOutputData>,

    tracker: UiStateTracker,
    currently_drawn_cable: Option<CurrentlyDrawnCable>,
}
impl<'a> CubedawNodeUiContext<'a> {
    pub fn new(
        id: Option<Id<NodeEntry>>,
        track_id: Id<Track>,
        node_data: &'a NodeEntry,
        ephemeral: &'a mut NodeEphemeralState,
        currently_drawn_cable: Option<CurrentlyDrawnCable>,
    ) -> Self {
        Self {
            node_id: id,
            track_id,
            node_data,

            node_ephemeral: ephemeral,
            inputs: Vec::new(),
            outputs: Vec::new(),

            tracker: UiStateTracker::new(),
            currently_drawn_cable,
        }
    }

    fn apply(&mut self, tracker: &mut UiStateTracker) {
        if self.node_id.is_some() {
            let old_num_inputs = self.node_data.inputs().len();
            let num_inputs = self.inputs.len();
            if num_inputs < old_num_inputs {
                for deleted_input in &self.node_data.inputs()[num_inputs..old_num_inputs] {
                    // do in reverse order because removing elements one-by-one from the vec is faster if you remove from last to first
                    for connection in deleted_input.connections.iter().rev() {
                        tracker.add_weak(cubedaw_command::patch::CableAddOrRemove::removal(
                            connection.id,
                            self.track_id,
                        ));
                    }
                }
            }
        }

        tracker.extend(self.tracker.take());
    }

    fn finish(self, node_rect: Rect) -> CubedawNodeUiContextResult {
        CubedawNodeUiContextResult {
            node_rect,
            inputs: self.inputs,
            outputs: self.outputs,
        }
    }
}

impl crate::node::NodeUiContext for CubedawNodeUiContext<'_> {
    fn input_ui(
        &mut self,
        ui: &mut egui::Ui,
        name: &str,
        options: crate::node::NodeInputUiOptions,
    ) {
        // the index of this current input.
        let input_index = self.inputs.len() as u32;
        let input = self.node_data.inputs().get(input_index as usize);

        let bias = match input {
            Some(input) => input.bias,
            None => options.default_value,
        };
        let mut new_bias = bias;

        let input_response = ui.add(
            DragValue::new(&mut new_bias)
                .name(Some(name))
                .interactable(options.interactable)
                .show_number_text(options.interactable)
                .range(options.range)
                .display_range(options.display_range)
                .display(options.display),
        );

        if let Some(id) = self.node_id {
            let command = cubedaw_command::node::NodeBiasChange::new(
                id,
                self.track_id,
                input_index,
                bias,
                new_bias,
            );
            if input_response.drag_stopped() || input_response.lost_focus() {
                // even if the value didn't change, if the user stops dragging it should be set...
                self.tracker.add(command);
            } else if new_bias != bias {
                // otherwise, if it did change, add a weak command to update the workers and whatnot
                self.tracker.add_weak(command);
            }
        }

        // render the cable connections
        let mut cable_connections = Vec::new();
        let mut virtual_index: Option<u32> = None;

        if let Some(input) = input {
            let mut connections: Vec<(bool, &CableConnection)> =
                input.connections.iter().map(|conn| (false, conn)).collect();

            if let Some(ref currently_drawn_cable) = self.currently_drawn_cable
                && let Some((
                    NodeSlotDescriptor::Input {
                        node,
                        input_index: other_input_index,
                        conn_index: Some(cable_index),
                    },
                    ref conn,
                )) = currently_drawn_cable.originally_attached
                && Some(node) == self.node_id
                && input_index == other_input_index
                && input
                    .connections
                    .get(cable_index as usize)
                    .is_none_or(|conn| conn.id != currently_drawn_cable.id)
            {
                // if the currently drawn cable refers to this input and the input doesn't exist, insert a virtual cable connection
                virtual_index = Some(cable_index);
                connections.insert(
                    cable_index as usize,
                    (currently_drawn_cable.tag == CableTag::Disconnected, conn),
                );
            }

            cable_connections.reserve_exact(connections.len());
            let mut indicator_top_y = 0.0;
            for (cable_index, &(is_virtual, conn)) in connections.iter().enumerate() {
                let cable_index = cable_index as u32;

                let multiplier = conn.multiplier;
                let mut new_multiplier = multiplier;

                // the little ╯ on the side
                let indicator_size = input_response.rect.height();

                let cable_connection_response = ui
                    .scope(|ui| {
                        if is_virtual {
                            ui.disable();
                        }
                        ui.set_max_width(ui.max_rect().width() - indicator_size);
                        ui.add(
                            DragValue::new(&mut new_multiplier)
                                .relative(true)
                                .display_range(Rangef::new(-1.0, 1.0))
                                .display(options.display),
                        )
                    })
                    .inner;

                let response_rect = cable_connection_response.rect;

                let indicator_rect = Rect::from_x_y_ranges(
                    response_rect.right()..=ui.max_rect().right(),
                    response_rect.y_range(),
                );
                if cable_index == 0 {
                    indicator_top_y = indicator_rect.top();
                }

                let indicator_stroke =
                    egui::Stroke::new(1.5, ui.visuals().widgets.inactive.bg_fill);
                ui.painter().with_clip_rect(indicator_rect).rect(
                    indicator_rect.translate(indicator_rect.size() * -0.5),
                    egui::Rounding {
                        se: 4.0,
                        ..Default::default()
                    },
                    egui::Color32::TRANSPARENT,
                    indicator_stroke,
                );

                if let Some(id) = self.node_id {
                    let command = cubedaw_command::node::NodeMultiplierChange::new(
                        id,
                        self.track_id,
                        input_index,
                        cable_index,
                        multiplier,
                        new_multiplier,
                    );
                    if cable_connection_response.drag_stopped()
                        || cable_connection_response.lost_focus()
                    {
                        self.tracker.add(command);
                    } else if new_multiplier != multiplier {
                        self.tracker.add_weak(command);
                    }
                }

                // the bar connecting the ╯s
                if cable_index == connections.len() as u32 - 1 && cable_index != 0 {
                    ui.painter().vline(
                        indicator_rect.center().x + 0.8, // why this constant? no idea, but it's what aligns the lines
                        indicator_top_y..=indicator_rect.top(),
                        indicator_stroke,
                    );
                }

                cable_connections.push(CubedawNodeUiContextCableConnectionData {
                    y_pos: cable_connection_response.rect.center().y,
                    multiplier: new_multiplier,
                    is_virtual,
                });
            }
        }

        self.inputs.push(CubedawNodeUiContextInputData {
            y_pos: input_response.rect.center().y,
            virtual_index,
            value: new_bias,
            cables: cable_connections,
        });
    }
    fn output_ui(&mut self, ui: &mut egui::Ui, name: &str) {
        let response = ui
            .with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.add(egui::Label::new(name))
            })
            .inner;

        self.outputs.push(CubedawNodeUiContextOutputData {
            y_pos: response.rect.center().y,
        });
    }
}

#[derive(Debug)]
struct CubedawNodeUiContextInputData {
    y_pos: f32,
    value: f32,

    // if there is a virtual cable connection, where is it located?
    virtual_index: Option<u32>,
    cables: Vec<CubedawNodeUiContextCableConnectionData>,
}
impl CubedawNodeUiContextInputData {
    // for non-virtual cable connections
    fn get(&self, mut conn_idx: u32) -> &CubedawNodeUiContextCableConnectionData {
        if let Some(virtual_index) = self.virtual_index
            && conn_idx >= virtual_index
        {
            conn_idx += 1;
        }
        &self.cables[conn_idx as usize]
    }
}
#[derive(Debug)]
struct CubedawNodeUiContextCableConnectionData {
    y_pos: f32,
    multiplier: f32,

    // used for when the user drags the end of an existing cable. the node slot should still be shown until the user releases the drag.
    is_virtual: bool,
}
#[derive(Debug)]
struct CubedawNodeUiContextOutputData {
    y_pos: f32,
}

#[derive(Debug)]
struct CubedawNodeUiContextResult {
    node_rect: Rect,

    inputs: Vec<CubedawNodeUiContextInputData>,
    outputs: Vec<CubedawNodeUiContextOutputData>,
}
impl CubedawNodeUiContextResult {
    fn get_input_pos(&self, input_index: u32, mut conn_index: Option<u32>) -> Pos2 {
        let input = &self.inputs[input_index as usize];
        if let Some(virtual_index) = input.virtual_index
            && let Some(conn_index) = conn_index.as_mut()
            && *conn_index >= virtual_index
        {
            *conn_index += 1;
        }
        self.get_input_pos_raw(input_index, conn_index)
    }
    /// Like `get_input_pos`, but doesn't take into account the virtual cable.
    fn get_input_pos_raw(&self, input_index: u32, conn_index: Option<u32>) -> Pos2 {
        let input = &self.inputs[input_index as usize];
        let y_pos = match conn_index {
            Some(idx) => input.cables[idx as usize].y_pos,
            None => input.y_pos,
        };
        Pos2 {
            x: self.node_rect.left(),
            y: y_pos,
        }
    }
    fn get_output_pos(&self, output_index: u32) -> Pos2 {
        let y_pos = self.outputs[output_index as usize].y_pos;
        Pos2 {
            x: self.node_rect.right(),
            y: y_pos,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeSlotDescriptor {
    Output {
        node: Id<NodeEntry>,
        output_index: u32,
    },
    Input {
        node: Id<NodeEntry>,
        input_index: u32,
        conn_index: Option<u32>,
    },
}
impl NodeSlotDescriptor {
    pub fn get_pos(self, data: &IdMap<NodeEntry, CubedawNodeUiContextResult>) -> Pos2 {
        match self {
            Self::Output { node, output_index } => {
                data.force_get(node).get_output_pos(output_index)
            }
            Self::Input {
                node,
                input_index,
                conn_index,
            } => data.force_get(node).get_input_pos(input_index, conn_index),
        }
    }
    pub fn get_pos_raw(self, data: &IdMap<NodeEntry, CubedawNodeUiContextResult>) -> Pos2 {
        match self {
            Self::Output { node, output_index } => {
                data.force_get(node).get_output_pos(output_index)
            }
            Self::Input {
                node,
                input_index,
                conn_index,
            } => data
                .force_get(node)
                .get_input_pos_raw(input_index, conn_index),
        }
    }
}
