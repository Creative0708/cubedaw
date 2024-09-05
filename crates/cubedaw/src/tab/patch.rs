use cubedaw_command::{node::NodeStateUpdate, patch::CableAddOrRemove};
use cubedaw_lib::{
    Cable, CableTag, Id, IdMap, Node as _, NodeData, NodeEntry, NodeInputUiOptions,
    NodeStateWrapper, Track,
};
use egui::{emath::TSTransform, pos2, Pos2, Rect, Vec2};

use crate::{
    command::node::{UiNodeAddOrRemove, UiNodeMove, UiNodeSelect},
    context::UiStateTracker,
    state::ui::NodeUiState,
    util::DragHandler,
    widget::DragValue,
};

pub struct PatchTab {
    id: Id<crate::app::Tab>,

    track_id: Option<Id<Track>>,

    transform: TSTransform,

    currently_held_node: Option<NodeData>,
    currently_drawn_cable: Option<NodeSlotDescriptor>,

    drag_handler: DragHandler,
}

fn transform_viewport(transform: TSTransform, viewport: Rect) -> TSTransform {
    TSTransform::new(
        transform.translation + viewport.center().to_vec2(),
        transform.scaling,
    )
}

impl crate::Screen for PatchTab {
    fn create(state: &cubedaw_lib::State, ui_state: &crate::UiState) -> Self
    where
        Self: Sized,
    {
        Self {
            id: Id::arbitrary(),

            track_id: ui_state.get_single_selected_track(),

            transform: TSTransform::IDENTITY,

            currently_held_node: None,
            currently_drawn_cable: None,

            drag_handler: DragHandler::new(),
        }
    }

    fn id(&self) -> Id<crate::app::Tab> {
        self.id
    }

    fn title(&self) -> egui::WidgetText {
        "Patch Tab".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
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
                    .show(ui.ctx(), |ui| {
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
                                    self.transform.translation +=
                                        hover_pos.to_vec2() * self.transform.scaling * (1.0 - zoom);
                                }
                                self.transform.scaling *= zoom;
                            }
                        }
                        let transform = transform_viewport(self.transform, screen_viewport);
                        let viewport = transform.inverse() * screen_viewport;

                        ui.ctx().set_transform_layer(layer_id, transform);
                        ui.ctx().set_sublayer(parent_layer_id, layer_id);

                        ui.set_clip_rect(viewport);
                        self.node_view(ctx, ui, viewport, viewport_interaction);
                    });
            } else {
                ui.with_layout(
                    egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                    |ui| {
                        ui.label("No track selected");
                    },
                );
            }
        });
    }

    fn drop(self: Box<Self>, egui_ctx: &egui::Context) {
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
    ) {
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
        painter.rect_filled(
            viewport,
            egui::Rounding::ZERO,
            ui.visuals().extreme_bg_color,
        );
        viewport_interaction.context_menu(|ui| {
            ui.menu_button("Add...", |ui| {
                let mut node_added: Option<Box<dyn NodeStateWrapper>> = None;
                // TODO make this a search bar
                if ui.button("Math").clicked() {
                    node_added = Some(Box::new(crate::node::math::MathNode::new_state(
                        Default::default(),
                    )));
                }
                if ui.button("Oscillator").clicked() {
                    node_added = Some(Box::new(
                        crate::node::oscillator::OscillatorNode::new_state(Default::default()),
                    ));
                }
                if ui.button("Note Output").clicked() {
                    node_added = Some(Box::new(
                        cubedaw_lib::builtin_nodes::NoteOutputNode::new_state(Default::default()),
                    ));
                }
                if ui.button("Track Output").clicked() {
                    node_added = Some(Box::new(
                        cubedaw_lib::builtin_nodes::TrackOutputNode::new_state(Default::default()),
                    ));
                }

                if let Some(node_added) = node_added {
                    let key = ctx.node_registry.get_resource_key_of(node_added.as_ref());
                    self.currently_held_node = Some(NodeData::new_disconnected(key, node_added));
                    ui.close_menu();
                }
            });
        });
        if viewport_interaction.secondary_clicked() {
            self.currently_held_node = None;
        }

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

        const DOT_SPACING: f32 = 16.0;
        const DOT_RADIUS: f32 = 1.5;

        // don't draw too many circles
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

        let cable_shapeidx = ui.painter().add(egui::Shape::Noop);

        let mut hovered_node_slot = None;
        let mut cable_drag_stopped = false;

        let result = self.drag_handler.handle(|prepared| {
            // nodes

            struct NodeSlotData {
                selected: bool,
                left: f32,
                right: f32,
                inputs: Box<[f32]>,
                outputs: Box<[f32]>,
            }
            impl NodeSlotData {
                fn get_input_pos(&self, input_index: u32) -> Pos2 {
                    Pos2::new(self.left, self.inputs[input_index as usize])
                }
                fn get_output_pos(&self, output_index: u32) -> Pos2 {
                    Pos2::new(self.right, self.outputs[output_index as usize])
                }
            }

            let currently_held_node_is_some = self.currently_held_node.is_some();
            let mut node_slot_data: IdMap<NodeEntry, NodeSlotData> = IdMap::new();
            // what the heck is rustfmt doing
            let mut handle_node =
                |prepared: &mut crate::util::Prepared<(Id<Track>, Id<NodeEntry>)>,
                 ui: &mut egui::Ui,
                 node_data: &NodeEntry,
                 node_id: Option<Id<NodeEntry>>,
                 node_ui: &NodeUiState,
                 tracker: &mut UiStateTracker| {
                    let real_node_data = node_id
                        .map(|node_id| (node_id, track_ephemeral.nodes.force_get_mut(node_id)));

                    let pos = if node_ui.selected
                        && let Some(offset) = prepared.movement()
                    {
                        node_ui.pos + offset
                    } else {
                        node_ui.pos
                    };

                    let node_max_rect = Rect::from_x_y_ranges(
                        pos.x..=pos.x + node_ui.width,
                        pos.y..=if let Some((_, ref node_ephemeral)) = real_node_data {
                            pos.y + node_ephemeral.size.y
                        } else {
                            // would be f32::INFINITY but egui needs a finite rect
                            viewport.bottom() + 128.0
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
                        && !prepared
                            .dragged_id()
                            .is_some_and(|id| Some(id.cast()) == node_id)
                    {
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

                    let mut ui_ctx = CubedawNodeUiContext::new(node_id, track_id, node_data);

                    let frame_rect;

                    // node frame
                    let mut frame_prepared = frame.begin(&mut frame_ui);
                    frame_prepared
                        .content_ui
                        .style_mut()
                        .interaction
                        .selectable_labels = false;
                    {
                        if let Some((node_id, ref node_ephemeral)) = real_node_data {
                            let drag_response = frame_ui.allocate_rect(
                                Rect::from_min_size(node_ui.pos, node_ephemeral.size),
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
                        let mut inner_node_ui = node_state.clone();

                        // TODO add header colors
                        frame_prepared.content_ui.label(inner_node_ui.title());
                        frame_prepared.content_ui.separator();
                        inner_node_ui.ui(&mut frame_prepared.content_ui, &mut ui_ctx);

                        if *inner_node_ui != *node_state
                            && let Some(node_id) = node_id
                        {
                            tracker.add(NodeStateUpdate::new(
                                node_id,
                                track_id,
                                inner_node_ui,
                                ui_ctx.inputs.iter().map(|i| i.value).collect(),
                                node_data.inputs().iter().map(|i| i.bias).collect(),
                                ui_ctx.outputs.len() as u32,
                                node_data.outputs().len() as u32,
                            ));
                        }

                        frame_rect = frame_prepared.content_ui.min_rect() + frame.total_margin();
                        if let Some((_node_id, node_ephemeral)) = real_node_data {
                            frame_prepared.allocate_space(&mut frame_ui);

                            node_ephemeral.size = frame_rect.size();
                        }
                    }
                    frame_prepared.paint(&frame_ui);

                    // node inputs
                    for ((index, y_pos), is_output) in ui_ctx
                        .inputs
                        .iter()
                        .map(|i| i.y_pos)
                        .enumerate()
                        .zip(std::iter::repeat(false))
                        .chain(
                            ui_ctx
                                .outputs
                                .iter()
                                .map(|o| o.y_pos)
                                .enumerate()
                                .zip(std::iter::repeat(true)),
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
                                    Rect::from_min_size(pos, Vec2::ZERO)
                                        .expand(slot_radius + ui.input(|i| i.aim_radius())),
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
                                }
                            };

                            if response.drag_started() {
                                self.currently_drawn_cable = Some(slot_descriptor);
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

                    if let Some(node_id) = node_id {
                        node_slot_data.insert(
                            node_id,
                            NodeSlotData {
                                selected: node_ui.selected,
                                left: frame_rect.left(),
                                right: frame_rect.right(),
                                inputs: ui_ctx.inputs.iter().map(|x| x.y_pos).collect(),
                                outputs: ui_ctx.outputs.iter().map(|x| x.y_pos).collect(),
                            },
                        );
                    }

                    ui_ctx.finish()
                };
            for (node_id, node_data) in patch.nodes() {
                let node_ui = patch_ui.nodes.get(node_id).expect("nonexistent node ui");

                handle_node(
                    prepared,
                    ui,
                    node_data,
                    Some(node_id),
                    node_ui,
                    &mut ctx.tracker,
                );
            }

            // .take() is used to avoid doing an obvious unwrap when checking if the node should be placed.
            // if the node isn't placed, the node is put back into currently_held_node at the end.
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
                    );
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

            node_slot_data
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

        let node_slots = result.inner;

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

            let control_point_distance = (input_pos.x - output_pos.x).abs() * 0.5;

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

        for (_cable_id, cable) in patch.cables() {
            draw_cable(
                node_slots
                    .force_get(cable.input_node)
                    .get_output_pos(cable.input_output_index),
                node_slots
                    .force_get(cable.output_node)
                    .get_input_pos(cable.output_input_index),
                cable.tag,
            );
        }

        if let Some(currently_drawn_cable) = self.currently_drawn_cable {
            if let Some(pointer_pos) = pointer_pos {
                // TODO refactor. we check that one slot is an input and another is an output ok
                let mut viable_cable = match (hovered_node_slot, currently_drawn_cable) {
                    (
                        Some(NodeSlotDescriptor::Output {
                            node: input_node,
                            output_index,
                        }),
                        NodeSlotDescriptor::Input {
                            node: output_node,
                            input_index,
                        },
                    ) => Some(Cable {
                        input_node,
                        input_output_index: output_index,
                        output_node,
                        output_input_index: input_index,
                        output_multiplier_fac: 1.0,
                        tag: CableTag::Invalid,
                    }),
                    (
                        Some(NodeSlotDescriptor::Input {
                            node: output_node,
                            input_index,
                        }),
                        NodeSlotDescriptor::Output {
                            node: input_node,
                            output_index,
                        },
                    ) => Some(Cable {
                        input_node,
                        input_output_index: output_index,
                        output_node,
                        output_input_index: input_index,
                        output_multiplier_fac: 1.0,
                        tag: CableTag::Invalid,
                    }),
                    _ => None,
                };
                if let Some(ref mut cable) = viable_cable {
                    draw_cable(
                        node_slots
                            .force_get(cable.input_node)
                            .get_output_pos(cable.input_output_index),
                        node_slots
                            .force_get(cable.output_node)
                            .get_input_pos(cable.output_input_index),
                        patch.get_cable_tag_if_added(cable),
                    );
                } else {
                    match currently_drawn_cable {
                        NodeSlotDescriptor::Input { node, input_index } => {
                            let input_pos = node_slots.force_get(node).get_input_pos(input_index);
                            draw_cable(pointer_pos, input_pos, CableTag::Disconnected)
                        }
                        NodeSlotDescriptor::Output { node, output_index } => {
                            let output_pos =
                                node_slots.force_get(node).get_output_pos(output_index);
                            draw_cable(output_pos, pointer_pos, CableTag::Disconnected);
                        }
                    }
                }
                if cable_drag_stopped {
                    if let Some(viable_cable) = viable_cable {
                        ctx.tracker.add(CableAddOrRemove::addition(
                            Id::arbitrary(),
                            viable_cable,
                            track_id,
                        ));
                    }

                    self.currently_drawn_cable = None;
                }
            }
        }

        ui.painter()
            .set(cable_shapeidx, egui::Shape::Vec(cable_shapes));
    }
}

struct CubedawNodeUiContext<'a> {
    node_id: Option<Id<NodeEntry>>,
    track_id: Id<Track>,
    node_data: &'a NodeEntry,

    inputs: Vec<CubedawNodeUiContextInput>,
    outputs: Vec<CubedawNodeUiContextOutput>,

    tracker: UiStateTracker,
}
impl<'a> CubedawNodeUiContext<'a> {
    pub fn new(id: Option<Id<NodeEntry>>, track_id: Id<Track>, node_data: &'a NodeEntry) -> Self {
        Self {
            node_id: id,
            track_id,
            node_data,

            inputs: Vec::new(),
            outputs: Vec::new(),

            tracker: UiStateTracker::new(),
        }
    }

    fn apply(&mut self, tracker: &mut UiStateTracker) {
        if self.node_id.is_some() {
            let old_num_inputs = self.node_data.inputs().len();
            let num_inputs = self.inputs.len();
            if num_inputs < old_num_inputs {
                for deleted_input in &self.node_data.inputs()[num_inputs..old_num_inputs] {
                    // do in reverse order because removing elements one-by-one from the vec is faster if you remove from last to first
                    for &connection in deleted_input.connections.iter().rev() {
                        tracker.add_weak(cubedaw_command::patch::CableAddOrRemove::removal(
                            connection,
                            self.track_id,
                        ));
                    }
                }
            }
        }

        tracker.extend(self.tracker.take());
    }

    fn finish(self) -> CubedawNodeUiContextResult {
        CubedawNodeUiContextResult {
            inputs: self.inputs,
            outputs: self.outputs,
        }
    }
}

impl cubedaw_lib::NodeUiContext for CubedawNodeUiContext<'_> {
    fn input_ui(&mut self, ui: &mut egui::Ui, name: &str, options: NodeInputUiOptions) {
        let num_inputs = self.inputs.len();
        let input = self.node_data.inputs().get(num_inputs);

        let previous_value = match input {
            Some(input) => input.bias,
            None => options.default_value,
        };
        let mut value = previous_value;

        let response = ui.add(
            DragValue::new(&mut value)
                .name(Some(name))
                .display_range(options.display_range)
                .range(options.range)
                .display(options.display),
        );

        if let Some(id) = self.node_id {
            let command = cubedaw_command::node::NodeInputChange::new(
                id,
                self.track_id,
                num_inputs,
                previous_value,
                value,
            );
            if response.drag_stopped() || response.lost_focus() {
                self.tracker.add(command);
            } else if previous_value != value {
                self.tracker.add_weak(command);
            }
        }

        self.inputs.push(CubedawNodeUiContextInput {
            y_pos: response.rect.center().y,
            value,
        });
    }
    fn output_ui(&mut self, ui: &mut egui::Ui, name: &str) {
        let response = ui
            .with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                ui.add(egui::Label::new(name))
            })
            .inner;

        self.outputs.push(CubedawNodeUiContextOutput {
            y_pos: response.rect.center().y,
        });
    }
}
struct CubedawNodeUiContextInput {
    y_pos: f32,
    value: f32,
}
struct CubedawNodeUiContextOutput {
    y_pos: f32,
}

struct CubedawNodeUiContextResult {
    inputs: Vec<CubedawNodeUiContextInput>,
    outputs: Vec<CubedawNodeUiContextOutput>,
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
    },
}
