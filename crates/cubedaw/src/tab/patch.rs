use std::iter;

use anyhow::Result;

use crate::{
    command::{node::NodeStateUpdate, patch::CableAddOrRemove},
    math,
};
use cubedaw_lib::{Buffer, Cable, CableConnection, CableTag, Id, IdMap, Node, NodeData, Track};
use egui::{
    Align, Area, CentralPanel, Color32, CornerRadius, CursorIcon, Direction, Frame, Layout, Pos2,
    Rangef, Rect, Response, Sense, Shape, Stroke, Ui, UiBuilder, Vec2, WidgetText,
    emath::TSTransform,
    epaint::{CubicBezierShape, PathStroke},
    layers::ShapeIdx,
    pos2, vec2,
};
use resourcekey::ResourceKey;

use crate::{
    command::node::NodeAddOrRemove,
    context::UiStateTracker,
    state::{ephemeral::NodeEphemeralState, ui::NodeUiState},
    util::Select,
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
#[derive(Debug, Clone)]
struct CurrentlyDrawnCable {
    /// Id of the cable. This is usually `Id::arbitrary()`, but in some circumstances is the id of a previously existing cable.
    pub id: Id<Cable>,

    /// The node slot that this cable is attached to. The other side is the mouse cursor.
    pub attached: NodeSlotDescriptor,

    /// The input node slot that this was originally attached to, or `None` if this was a just-created cable. Used when the user drags the end of an existing cable.
    pub originally_attached: Option<(NodeSlotDescriptor, CableConnection)>,

    /// If the user drags the end of this cable over another cable, the original cable is replaced. This holds the original cable in case the user moves their cursor away from the slot, making the original cable appear again.
    pub cable_that_this_replaces: Option<(Id<Cable>, Cable, CableConnection)>,

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

    fn title(&self) -> WidgetText {
        "Patch Tab".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut Ui) -> Result<()> {
        let Self { track_id, id, .. } = *self;

        CentralPanel::default()
            .show_inside(ui, |ui| -> Result<()> {
                let parent_layer_id = ui.layer_id();
                let area_id = parent_layer_id.id.with((parent_layer_id.order, id));
                let area = Area::new(area_id)
                    .movable(false)
                    .order(parent_layer_id.order);

                if track_id.is_some() {
                    let screen_viewport = ui.max_rect();
                    let transform = transform_viewport(self.transform, screen_viewport);

                    // we use an area here because it's the only way to render something with custom transforms above another layer.
                    // kinda jank (and there doesn't seem to be a way to delete an area), but oh well.
                    area.constrain_to(screen_viewport)
                        .show(ui.ctx(), |ui| -> Result<()> {
                            let layer_id = ui.layer_id();

                            // handle panning/zoom
                            let viewport = transform.inverse() * screen_viewport;
                            ui.set_clip_rect(viewport);
                            let viewport_interaction = ui.interact(
                                viewport,
                                layer_id.id.with("patch_move"),
                                Sense::click_and_drag(),
                            );
                            if viewport_interaction.contains_pointer() {
                                let (scroll_delta, zoom) =
                                    ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));

                                let zoom = (self.transform.scaling * zoom).clamp(0.2, 4.0)
                                    / self.transform.scaling;

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

                            let mut prepared = Prepared::new(
                                self,
                                ctx,
                                ui,
                                viewport_interaction,
                                ui.input(|i| i.pointer.hover_pos())
                                    .map(|pos| transform.inverse() * pos),
                            );

                            prepared.background(ui, ctx);
                            prepared.show_add_node_menu(ui, ctx);
                            // cables are rendered below the nodes; save a ShapeIdx for them!
                            let cable_shapeidx = ui.painter().add(Shape::Noop);
                            let node_results = prepared.handle_nodes(ui, ctx)?;
                            let cable_result =
                                prepared.do_cable_interactions(ui, ctx, &node_results);
                            prepared.draw_cables(
                                ui,
                                ctx,
                                &node_results,
                                cable_result,
                                cable_shapeidx,
                            );

                            Ok(())
                        })
                        .inner?;
                } else {
                    ui.with_layout(
                        Layout::centered_and_justified(Direction::LeftToRight),
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
}

impl PatchTab {
    pub fn select_track(&mut self, track_id: Option<Id<Track>>) {
        self.track_id = track_id;
    }
}

struct Prepared<'tab, 'ctx> {
    tab: &'tab mut PatchTab,

    track_id: Id<Track>,
    patch: &'ctx cubedaw_lib::Patch,
    patch_ui: &'ctx crate::state::ui::PatchUiState,

    viewport: Rect,
    viewport_interaction: Response,
    // i don't know why this is necessary but removing it breaks everything apparently
    // todo!()
    pointer_pos: Option<Pos2>,
}

impl<'tab, 'ctx> Prepared<'tab, 'ctx> {
    pub fn new(
        tab: &'tab mut PatchTab,
        ctx: &mut crate::Context<'ctx>,
        ui: &mut Ui,
        viewport_interaction: Response,
        pointer_pos: Option<Pos2>,
    ) -> Self {
        let viewport = ui.clip_rect();

        let track_id = tab.track_id.expect("unreachable");

        Self {
            tab,
            track_id,
            patch: &ctx.state.tracks.force_get(track_id).patch,
            patch_ui: &ctx.ui_state.tracks.force_get(track_id).patch,

            viewport,
            viewport_interaction,
            pointer_pos,
        }
    }

    pub fn background(&mut self, ui: &mut Ui, _ctx: &mut crate::Context<'ctx>) {
        let Self {
            viewport,
            ref viewport_interaction,
            ..
        } = *self;

        let painter = ui.painter();

        painter.rect_filled(viewport, CornerRadius::ZERO, ui.visuals().extreme_bg_color);

        const DOT_SPACING: f32 = 24.0;
        const DOT_RADIUS: f32 = 3.0;

        // don't draw too many dots
        const MAX_DOTS: f32 = 10000.0;
        let num_dots = viewport.area() / (DOT_SPACING * DOT_SPACING);
        if num_dots < MAX_DOTS {
            for x in math::frange_snapped(
                viewport.left() - DOT_RADIUS,
                viewport.right() + DOT_RADIUS,
                DOT_SPACING,
            ) {
                for y in math::frange_snapped(
                    viewport.top() - DOT_RADIUS,
                    viewport.bottom() + DOT_RADIUS,
                    DOT_SPACING,
                ) {
                    painter.circle_filled(
                        pos2(x, y),
                        DOT_RADIUS,
                        // cleanly fade out dots
                        ui.visuals()
                            .faint_bg_color
                            .gamma_multiply(f32::min(2.0 * (1.0 - num_dots / MAX_DOTS), 1.5)),
                    );
                }
            }
        }
    }
    pub fn show_add_node_menu(&mut self, _ui: &mut Ui, ctx: &mut crate::Context<'ctx>) {
        let Self {
            tab: &mut ref mut tab,
            ref viewport_interaction,
            ..
        } = *self;

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
                    node_added = Some(resourcekey::literal!("builtin:output"));
                }
                if ui.button("Track Output").clicked() {
                    node_added = Some(resourcekey::literal!("builtin:track_output"));
                }

                if let Some(key) = node_added {
                    ui.close_menu();

                    let entry = ctx.node_registry.get(&key).expect("wut");
                    tab.currently_held_node = Some(NodeData::new_disconnected(
                        key,
                        entry
                            .ui
                            .create(&crate::node::NodeCreationContext::default())
                            .as_ref()
                            .into(),
                    ));
                }
            });
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn ui_node(
        &mut self,
        ui: &mut Ui,
        ctx: &mut crate::Context<'ctx>,
        patch_ephemeral_node_map: &mut IdMap<Node, NodeEphemeralState>,
        prepared: &mut crate::util::Prepared<Id<Node>, impl Fn(Pos2) -> Pos2>,
        node_data: &Node,
        node_id: Option<Id<Node>>,
        node_ui: &NodeUiState,
    ) -> Result<(
        CubedawNodeUiContextResult,
        Option<InteractedNodeSlot>,
        Option<InteractedNodeSlot>,
    )> {
        let Self {
            ref mut tab,

            viewport,

            track_id,
            ..
        } = *self;

        // Some(node_id, ephemeral_state) if node actually exists, None if the node is just there for rendering
        // (e.g. the user is adding a node and is choosing where to place it)
        let mut real_node_data = node_id.map(|node_id| {
            (
                node_id,
                patch_ephemeral_node_map.get_mut_or_insert_default(node_id),
            )
        });

        let pos = if node_ui.select.is()
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

        let mut frame_ui = ui.new_child(
            UiBuilder::new()
                .max_rect(node_max_rect)
                .id_salt(node_id.unwrap_or(Id::new("currently_held_node"))),
        );
        if tab.currently_held_node.is_some() {
            frame_ui.disable();
        }
        if node_id.is_some()
            && !node_max_rect.intersects(viewport)
            && prepared.dragged_thing() != node_id.map(Id::cast)
        {
            // node isn't visible, hide it
            frame_ui.set_invisible();
        }
        frame_ui.spacing_mut().item_spacing = vec2(8.0, 4.0);

        let mut frame = Frame::window(ui.style()).inner_margin(8.0);
        if node_ui.select.is() {
            // TODO actually implement selection colors/strokes
            frame.stroke = Stroke::new(frame.stroke.width * 1.2, Color32::from_gray(96));
            frame.fill = Color32::from_gray(32);
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
            tab.currently_drawn_cable.clone(),
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
                    Sense::click_and_drag(),
                );
                prepared.process_interaction(
                    node_id.cast(),
                    &drag_response,
                    node_id,
                    node_ui.select,
                );
            }
            let node_state = node_data.data.inner.as_ref();
            let mut node_state_copy: Box<Buffer> = node_state.into();

            // TODO add header colors
            let node_thingy = ctx
                .node_registry
                .get(&node_data.data.key)
                .unwrap_or_else(|| panic!("unknown node encountered: {:?}", node_data.data.key))
                .ui
                .as_ref();

            frame_prepared
                .content_ui
                .label(node_thingy.title(node_state, ctx)?);
            frame_prepared.content_ui.separator();
            node_thingy.ui(
                &mut node_state_copy,
                &mut frame_prepared.content_ui,
                &mut ui_ctx,
            )?;

            if *node_state_copy != *node_state
                && let Some(node_id) = node_id
            {
                ctx.tracker.add(NodeStateUpdate::new(
                    node_id,
                    track_id,
                    node_state_copy,
                    ui_ctx.inputs.iter().map(|i| i.value).collect(),
                    node_data.inputs().iter().map(|i| i.bias).collect(),
                    ui_ctx.outputs.len() as u32,
                    node_data.outputs().len() as u32,
                ));
            }

            frame_rect = frame_prepared.content_ui.min_rect() + frame.total_margin();
            frame_prepared.allocate_space(&mut frame_ui);
            ui_ctx.node_ephemeral.size = frame_rect.size();
        }
        frame_prepared.paint(&frame_ui);

        ui_ctx.apply(&mut ctx.tracker);

        let result = ui_ctx.finish(frame_rect);

        let (dragged_node_slot, hovered_node_slot) = ui
            .add_enabled_ui(self.tab.currently_held_node.is_none(), |ui| {
                self.handle_node_slots_for(ui, node_id, &result)
            })
            .inner;

        Ok((result, dragged_node_slot, hovered_node_slot))
    }

    fn handle_node_slots_for(
        &mut self,
        ui: &mut Ui,
        node_id: Option<Id<Node>>,
        node_result: &CubedawNodeUiContextResult,
    ) -> (Option<InteractedNodeSlot>, Option<InteractedNodeSlot>) {
        let CubedawNodeUiContextResult { node_rect, .. } = node_result;

        let mut dragged_node_slot = None;
        let mut hovered_node_slot = None;

        // node slots
        // index: either input index or output index, depending on is_output
        // y_pos: screen y pos
        // cable_index: for inputs, either Some(the 0-based index of the cable this is connected to) or None for not being connected to a cable. for outputs, unused.
        // is_output: duh
        for ((index, (y_pos, cable_index)), is_output) in node_result
            .inputs
            .iter()
            .enumerate()
            .flat_map(|(idx, input)| {
                iter::once((idx, (input.input_y_pos, None))).chain(
                    input
                        .cables
                        .iter()
                        .enumerate()
                        .map(move |(cable_idx, cable_input)| {
                            (idx, (cable_input.y_pos, Some(cable_idx as u32)))
                        }),
                )
            })
            .zip(iter::repeat(false))
            .chain(
                node_result
                    .outputs
                    .iter()
                    .map(|o| (o.y_pos, None))
                    .enumerate()
                    .zip(iter::repeat(true)),
            )
        {
            let pos = Pos2 {
                x: if is_output {
                    node_rect.right()
                } else {
                    node_rect.left()
                },
                y: y_pos,
            };

            // TODO add configurable styles for this
            let slot_radius = 4.0;

            let response = ui
                .allocate_rect(
                    Rect::from_min_size(pos, Vec2::ZERO)
                        .expand(slot_radius + 4.0 + ui.input(|i| i.aim_radius())),
                    Sense::drag(),
                )
                .on_hover_cursor(CursorIcon::PointingHand);

            let hovered = response.contains_pointer();

            if let Some(node_id) = node_id {
                // handle node slot interactions
                let slot_descriptor = if is_output {
                    NodeSlotDescriptor::Output {
                        node_id,
                        output_index: index as u32,
                    }
                } else {
                    NodeSlotDescriptor::Input {
                        node_id,
                        input_index: index as u32,
                        conn_index: cable_index,
                    }
                };

                if response.dragged() || response.drag_stopped() {
                    dragged_node_slot = Some(InteractedNodeSlot {
                        descriptor: slot_descriptor,
                        response: response.clone(),
                    });
                }
                if response.contains_pointer() {
                    hovered_node_slot = Some(InteractedNodeSlot {
                        descriptor: slot_descriptor,
                        response,
                    });
                }
            }

            let visuals = if hovered {
                ui.visuals().widgets.hovered
            } else {
                ui.visuals().widgets.noninteractive
            };
            let slot_fill = visuals.bg_fill;
            let slot_stroke = visuals.bg_stroke;

            ui.painter()
                .circle(pos, slot_radius, slot_fill, slot_stroke);
        }

        (dragged_node_slot, hovered_node_slot)
    }

    fn handle_nodes(&mut self, ui: &mut Ui, ctx: &mut crate::Context<'ctx>) -> Result<NodeResults> {
        let Self {
            track_id,
            patch,
            patch_ui,
            ..
        } = *self;

        // cursed hack to satisfy the borrow checker... i mean i guess it makes sense but jeez
        let mut track_ephem = ctx.ephemeral_state.tracks.take(track_id);
        let result = track_ephem.patch.node_drag.handle::<fn(Pos2) -> Pos2, _>(
            |pos| pos,
            |prepared| -> Result<_> {
                if self.viewport_interaction.clicked() {
                    prepared.deselect_all();
                }

                let mut dragged_node_slot: Option<InteractedNodeSlot> = None;
                let mut hovered_node_slot: Option<InteractedNodeSlot> = None;

                let mut node_results_map: IdMap<Node, CubedawNodeUiContextResult> = IdMap::new();

                // nodes
                if self.viewport_interaction.secondary_clicked() {
                    self.tab.currently_held_node = None;
                }

                for (node_id, node_data) in patch.nodes() {
                    let node_ui = patch_ui.nodes.get(node_id).expect("nonexistent node ui");

                    let (result, dragged_node_slot_for_this_node, hovered_node_slot_for_this_node) =
                        self.ui_node(
                            ui,
                            ctx,
                            &mut track_ephem.patch.nodes,
                            prepared,
                            node_data,
                            Some(node_id),
                            node_ui,
                        )?;

                    dragged_node_slot = dragged_node_slot.or(dragged_node_slot_for_this_node);
                    hovered_node_slot = hovered_node_slot.or(hovered_node_slot_for_this_node);

                    node_results_map.insert(node_id, result);
                }

                if let Some(hover_pos) = self.viewport_interaction.hover_pos()
                    && let Some(node_data) = self.tab.currently_held_node.take()
                {
                    ui.ctx().set_cursor_icon(CursorIcon::AllScroll);
                    let fake_entry = Node::new(node_data, 0, 0);
                    let (result, ..) = self.ui_node(
                        ui,
                        ctx,
                        &mut track_ephem.patch.nodes,
                        prepared,
                        &fake_entry,
                        None,
                        &NodeUiState {
                            select: Select::Select,
                            pos: hover_pos,
                            width: 128.0,
                        },
                    )?;
                    let node_data = fake_entry.data;
                    if self.viewport_interaction.clicked() {
                        // place the node
                        ctx.tracker.add(NodeAddOrRemove::addition(
                            Id::arbitrary(),
                            node_data,
                            result.inputs.into_iter().map(|input| input.value).collect(),
                            result.outputs.len() as u32,
                            track_id,
                            NodeUiState {
                                select: Select::Select,
                                pos: hover_pos,
                                width: 128.0, // TODO impl node widths
                            },
                        ))
                    } else if self.viewport_interaction.secondary_clicked() {
                        // do nothing; since we're never setting currently_held_node to Some(_) after the take(), this deletes the node
                    } else {
                        self.tab.currently_held_node = Some(node_data);
                    }
                }

                Ok(NodeResults {
                    results: node_results_map,
                    dragged_node_slot,
                    hovered_node_slot,
                })
            },
        )?;

        ctx.ephemeral_state.tracks.insert(track_id, track_ephem);

        Ok(result)
    }

    fn do_cable_interactions(
        &mut self,
        ui: &mut Ui,
        ctx: &mut crate::Context<'ctx>,
        node_results: &NodeResults,
    ) -> Option<CableInteractionResult> {
        // this is gonna be used later, i can feel it
        // TODO: remove this line
        let _ = ui;

        let Self {
            ref mut tab,
            patch,
            track_id,
            pointer_pos,
            ..
        } = *self;
        let NodeResults {
            dragged_node_slot,
            hovered_node_slot,
            ..
        } = node_results;

        let tracker = &mut ctx.tracker;

        if let Some(InteractedNodeSlot {
            descriptor,
            ref response,
        }) = *dragged_node_slot
            && response.drag_started()
        {
            let node_data = patch.node_entry(descriptor.node_id()).expect("todo!()");

            match descriptor {
                NodeSlotDescriptor::Input {
                    conn_index: Some(conn_index),
                    input_index,
                    ..
                } if let Some(&(cable_id, ref conn)) = node_data.inputs()[input_index as usize]
                    .connections
                    .get(conn_index as usize) =>
                {
                    // if the slot is an input and there already is a cable there, take control of it
                    let cable = patch.cable(cable_id).expect("unreachable");

                    tab.currently_drawn_cable = Some(CurrentlyDrawnCable {
                        id: cable_id,
                        attached: NodeSlotDescriptor::Output {
                            node_id: cable.input_node,
                            output_index: cable.input_output_index,
                        },
                        originally_attached: Some((descriptor, conn.clone())),
                        cable_that_this_replaces: None,

                        tag: cable.input_node(patch).tag().cable_tag_for_output(),
                    });

                    // no need to remove the cable now; the code below will automatically remove the cable when it's not connected
                }
                _ => {
                    // create a new cable
                    tab.currently_drawn_cable = Some(CurrentlyDrawnCable {
                        id: Id::arbitrary(),
                        attached: descriptor,
                        originally_attached: None,

                        cable_that_this_replaces: None,
                        tag: CableTag::Disconnected,
                    });
                }
            }

            // add a strong command to allow for possible deletion later (so we don't delete another state command accidentally)
            tracker.add(crate::command::Noop);
        }

        let mut result = None;

        if let Some(pointer_pos) = pointer_pos {
            let hovered_node_slot_descriptor = hovered_node_slot
                .as_ref()
                .map(|node_slot| node_slot.descriptor);
            if let Some(mut currently_drawn_cable) = tab.currently_drawn_cable.take() {
                let viable_cable =
                    match (hovered_node_slot_descriptor, currently_drawn_cable.attached) {
                        (
                            Some(NodeSlotDescriptor::Output {
                                node_id: input_node,
                                output_index,
                            }),
                            NodeSlotDescriptor::Input {
                                node_id: output_node,
                                input_index,
                                conn_index: cable_index,
                            },
                        )
                        | (
                            Some(NodeSlotDescriptor::Input {
                                node_id: output_node,
                                input_index,
                                conn_index: cable_index,
                            }),
                            NodeSlotDescriptor::Output {
                                node_id: input_node,
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
                    } else if let Some(cable_index) = cable_index &&
                        let Some(&(cable_id, ref conn)) = cable
                            .node_input(patch)
                            .connections
                            .get(cable_index as usize)

                        // don't delete the cable at this index if it's where the cable is originally attached!
                        && currently_drawn_cable.originally_attached.as_ref().is_none_or(
                            |(descriptor, _)| {
                                descriptor
                                    != &NodeSlotDescriptor::Input {
                                        node_id: cable.output_node,
                                        input_index: cable.output_input_index,
                                        conn_index: Some(cable_index),
                                    }
                            },
                        )
                    {
                        currently_drawn_cable.cable_that_this_replaces = Some((
                            cable_id,
                            patch.cable(cable_id).expect("invalid patch").clone(),
                            conn.clone(),
                        ));
                        tracker.add_weak(CableAddOrRemove::removal(cable_id, track_id));

                        tracker.add_weak(CableAddOrRemove::addition(
                            currently_drawn_cable.id,
                            cable,
                            conn.clone(),
                            track_id,
                        ));
                    } else {
                        // was there an original node slot? yes? use the old cable conn then
                        let cable_conn = if let Some((_, ref cable_connection)) =
                            currently_drawn_cable.originally_attached
                        {
                            cable_connection.clone()
                        } else {
                            Default::default()
                        };

                        tracker.add_weak(CableAddOrRemove::addition(
                            currently_drawn_cable.id,
                            cable,
                            cable_conn,
                            track_id,
                        ));
                    }
                } else {
                    currently_drawn_cable.tag = CableTag::Disconnected;

                    if currently_drawn_cable_exists_in_patch {
                        tracker.add_weak(CableAddOrRemove::removal(
                            currently_drawn_cable.id,
                            track_id,
                        ));
                        if let Some((cable_id, cable, conn)) =
                            currently_drawn_cable.cable_that_this_replaces.take()
                        {
                            tracker.add_weak(CableAddOrRemove::addition(
                                cable_id, cable, conn, track_id,
                            ));
                        }
                    }
                }
                if should_render_currently_drawn_cable {
                    let attached_pos = currently_drawn_cable.attached.get_pos(node_results);
                    result = Some(match currently_drawn_cable.attached {
                        NodeSlotDescriptor::Input { .. } => {
                            let output_pos = match hovered_node_slot_descriptor {
                                Some(slot @ NodeSlotDescriptor::Output { .. }) => {
                                    slot.get_pos_raw(node_results)
                                }
                                _ => pointer_pos,
                            };
                            CableInteractionResult {
                                start_pos: attached_pos,
                                end_pos: output_pos,
                                tag: currently_drawn_cable.tag,
                            }
                        }
                        NodeSlotDescriptor::Output { .. } => {
                            let input_pos = match hovered_node_slot_descriptor {
                                Some(slot @ NodeSlotDescriptor::Input { .. }) => {
                                    slot.get_pos_raw(node_results)
                                }
                                _ => pointer_pos,
                            };
                            CableInteractionResult {
                                start_pos: input_pos,
                                end_pos: attached_pos,
                                tag: currently_drawn_cable.tag,
                            }
                        }
                    });
                }
                if dragged_node_slot
                    .as_ref()
                    .is_some_and(|node_slot| node_slot.response.drag_stopped())
                {
                    // did it actually do anything? no? guess all those commands were for nothing then. delete the commands
                    let was_added = currently_drawn_cable.originally_attached.is_none()
                        && currently_drawn_cable_exists_in_patch;
                    let was_deleted = currently_drawn_cable.originally_attached.is_some()
                        && !currently_drawn_cable_exists_in_patch;
                    let was_moved = currently_drawn_cable
                        .originally_attached
                        .as_ref()
                        .is_some_and(|(node_slot, _)| {
                            Some(*node_slot)
                                != dragged_node_slot.as_ref().map(|slot| slot.descriptor)
                        });
                    if !(was_added || was_deleted || was_moved) {
                        tracker.delete_last_command();
                    }
                } else {
                    tab.currently_drawn_cable = Some(currently_drawn_cable);
                }
            }
        }
        result
    }

    fn draw_cables(
        &mut self,
        ui: &mut Ui,
        _ctx: &mut crate::Context<'ctx>,
        node_results: &NodeResults,
        cable_result: Option<CableInteractionResult>,
        shapeidx: ShapeIdx,
    ) {
        let Self {
            viewport, patch, ..
        } = *self;

        // cables

        let mut cable_shapes: Vec<Shape> = Vec::new();
        let mut draw_cable = |input_pos: Pos2, output_pos: Pos2, tag: CableTag| {
            if !viewport.intersects(Rect::from_points(&[input_pos, output_pos])) {
                return;
            }

            let mut control_point_distance = (input_pos.x - output_pos.x).abs() * 0.5;
            const MIN_BEZIER_DISTANCE: f32 = 70.0;
            if control_point_distance.abs() < MIN_BEZIER_DISTANCE {
                control_point_distance = MIN_BEZIER_DISTANCE.copysign(control_point_distance);
            }

            let base_shape = CubicBezierShape {
                points: [
                    output_pos,
                    output_pos + Vec2::new(control_point_distance, 0.0),
                    input_pos - Vec2::new(control_point_distance, 0.0),
                    input_pos,
                ],
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: PathStroke::NONE,
            };

            let mut bezier_with_stroke = |stroke: Stroke| {
                cable_shapes.push(
                    CubicBezierShape {
                        stroke: stroke.into(),
                        ..base_shape
                    }
                    .into(),
                )
            };

            match tag {
                CableTag::Invalid => {
                    bezier_with_stroke(Stroke::new(4.0, ui.visuals().error_fg_color))
                }
                CableTag::Monophonic => {
                    bezier_with_stroke(Stroke::new(4.0, Color32::from_gray(128)))
                }
                CableTag::Multiphonic => {
                    // draw several beziers on top of each other to make it look like there's multiple cables
                    bezier_with_stroke(Stroke::new(10.0, Color32::from_gray(100)));
                    bezier_with_stroke(Stroke::new(
                        6.0,
                        ui.visuals().widgets.noninteractive.bg_fill,
                    ));
                    bezier_with_stroke(Stroke::new(2.0, Color32::from_gray(128)));
                }
                CableTag::Disconnected => {
                    bezier_with_stroke(Stroke::new(4.0, Color32::from_gray(60)))
                }
            }
        };

        for (_cable_id, cable) in patch.cables() {
            draw_cable(
                node_results
                    .results
                    .force_get(cable.output_node)
                    .get_input_pos(cable.output_input_index, Some(cable.output_cable_index)),
                node_results
                    .results
                    .force_get(cable.input_node)
                    .get_output_pos(cable.input_output_index),
                cable.tag,
            );
        }

        if let Some(CableInteractionResult {
            start_pos,
            end_pos,
            tag,
        }) = cable_result
        {
            draw_cable(start_pos, end_pos, tag);
        }

        ui.painter().set(shapeidx, Shape::Vec(cable_shapes));
    }
}

struct CubedawNodeUiContext<'a> {
    node_id: Option<Id<Node>>,
    track_id: Id<Track>,
    node_data: &'a Node,

    node_ephemeral: &'a mut NodeEphemeralState,
    inputs: Vec<CubedawNodeUiContextInputData>,
    outputs: Vec<CubedawNodeUiContextOutputData>,

    tracker: UiStateTracker,
    currently_drawn_cable: Option<CurrentlyDrawnCable>,
}
impl<'a> CubedawNodeUiContext<'a> {
    pub fn new(
        id: Option<Id<Node>>,
        track_id: Id<Track>,
        node_data: &'a Node,
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
                    for &(cable_id, _) in deleted_input.connections.iter().rev() {
                        tracker.add_weak(crate::command::patch::CableAddOrRemove::removal(
                            cable_id,
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
    fn input_ui(&mut self, ui: &mut Ui, name: &str, options: crate::node::NodeInputUiOptions) {
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
                .display(options.display)
                .extra(options.extra),
        );

        if let Some(id) = self.node_id {
            let command = crate::command::node::NodeBiasChange::new(
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
            let mut connections: Vec<(bool, &CableConnection)> = input
                .connections
                .iter()
                .map(|(_, conn)| (false, conn))
                .collect();

            if let Some(ref currently_drawn_cable) = self.currently_drawn_cable
                && let Some((
                    NodeSlotDescriptor::Input {
                        node_id: node,
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
                    .is_none_or(|&(id, _)| id != currently_drawn_cable.id)
            {
                // if the currently drawn cable refers to this input and the connection doesn't exist, insert a virtual cable connection
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

                let indicator_stroke = Stroke::new(1.5, ui.visuals().widgets.inactive.bg_fill);

                ui.painter().with_clip_rect(indicator_rect).rect_stroke(
                    indicator_rect.translate(indicator_rect.size() * -0.5),
                    CornerRadius {
                        se: 4,
                        ..Default::default()
                    },
                    indicator_stroke,
                    egui::StrokeKind::Inside,
                );

                if let Some(id) = self.node_id {
                    let command = crate::command::node::NodeMultiplierChange::new(
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
            input_y_pos: input_response.rect.center().y,
            virtual_index,
            value: new_bias,
            cables: cable_connections,
        });
    }
    fn output_ui(&mut self, ui: &mut Ui, name: &str) {
        let response = ui
            .with_layout(Layout::right_to_left(Align::Min), |ui| ui.label(name))
            .inner;

        self.outputs.push(CubedawNodeUiContextOutputData {
            y_pos: response.rect.center().y,
        });
    }
}

#[derive(Debug)]
struct CubedawNodeUiContextInputData {
    input_y_pos: f32,
    value: f32,

    /// If there is a virtual cable connection, where is it located?
    virtual_index: Option<u32>,
    /// List of cables for this input. This includes the virtual cable; be careful when indexing!
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
            None => input.input_y_pos,
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

struct NodeResults {
    pub results: IdMap<Node, CubedawNodeUiContextResult>,

    pub dragged_node_slot: Option<InteractedNodeSlot>,
    pub hovered_node_slot: Option<InteractedNodeSlot>,
}
impl NodeResults {
    // pub fn get_slot_descriptor_at(&self, pos: Pos2) -> Option<NodeSlotDescriptor> {
    //     for (node_id, node) in self.results {
    //         for input in &node.inputs {
    //             if input.input_y_pos
    //         }
    //     }
    // }
}

#[derive(Debug)]
struct InteractedNodeSlot {
    pub descriptor: NodeSlotDescriptor,
    pub response: Response,
}

#[derive(Debug)]
struct CableInteractionResult {
    start_pos: Pos2,
    end_pos: Pos2,
    tag: CableTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeSlotDescriptor {
    Input {
        node_id: Id<Node>,
        input_index: u32,
        conn_index: Option<u32>,
    },
    Output {
        node_id: Id<Node>,
        output_index: u32,
    },
}
impl NodeSlotDescriptor {
    // pub fn of_cable_input(cable: &Cable) -> Self {
    //     Self::Input {
    //         node_id: cable.output_node,
    //         input_index: cable.output_input_index,
    //         conn_index: Some(cable.output_cable_index),
    //     }
    // }

    pub fn node_id(self) -> Id<Node> {
        match self {
            Self::Input { node_id, .. } => node_id,
            Self::Output { node_id, .. } => node_id,
        }
    }

    pub fn get_pos(self, results: &NodeResults) -> Pos2 {
        match self {
            Self::Output {
                node_id: node,
                output_index,
            } => results.results.force_get(node).get_output_pos(output_index),
            Self::Input {
                node_id: node,
                input_index,
                conn_index,
            } => results
                .results
                .force_get(node)
                .get_input_pos(input_index, conn_index),
        }
    }
    pub fn get_pos_raw(self, results: &NodeResults) -> Pos2 {
        match self {
            Self::Output {
                node_id: node,
                output_index,
            } => results.results.force_get(node).get_output_pos(output_index),
            Self::Input {
                node_id: node,
                input_index,
                conn_index,
            } => results
                .results
                .force_get(node)
                .get_input_pos_raw(input_index, conn_index),
        }
    }
}
