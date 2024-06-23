use cubedaw_command::{node::NodeUiUpdate, StateTracker};
use cubedaw_lib::{Id, NodeData, NodeInputUiOptions, NodeStateWrapper, Track};
use cubedaw_node::Node;
use egui::{
    emath::TSTransform, pos2, Align, Align2, Color32, FontId, Layout, Pos2, Rect, Rounding, Stroke,
    TextStyle, Vec2,
};

use crate::{
    command::node::{UiNodeAddOrRemove, UiNodeSelect},
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

    drag_handler: DragHandler,
}

fn transform_viewport(transform: TSTransform, viewport: Rect) -> TSTransform {
    TSTransform::new(
        transform.translation + viewport.center().to_vec2(),
        transform.scaling,
    )
}

impl crate::Screen for PatchTab {
    fn create(ctx: &mut crate::Context) -> Self
    where
        Self: Sized,
    {
        Self {
            id: Id::arbitrary(),

            track_id: ctx.ui_state.get_single_selected_track(),

            transform: TSTransform::IDENTITY,

            currently_held_node: None,

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
                // TODO there doesn't seem to be a way to make a layer consistently display on top of another layer
                // so we use another order as a workaround.
                let layer_id = egui::LayerId::new(egui::Order::Middle, self.id.into());

                let screen_viewport = ui.max_rect();
                let transform = transform_viewport(self.transform, screen_viewport);

                ui.with_layer_id(layer_id, |ui| {
                    let viewport = transform.inverse() * screen_viewport;
                    ui.set_clip_rect(viewport);
                    let viewport_interaction = ui.interact(
                        viewport,
                        layer_id.id.with("patch_move"),
                        egui::Sense::click_and_drag(),
                    );
                    if let Some(hover_pos) = viewport_interaction.hover_pos() {
                        let (scroll_delta, zoom) =
                            ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
                        if scroll_delta != Vec2::ZERO {
                            self.transform.translation += scroll_delta;
                        }
                        if zoom != 1.0 {
                            let zoom_center = hover_pos - viewport.center();

                            // the zoom center should stay at the same location after the transform
                            // pos * scale + t = pos * (scale * zoom) + new_t
                            // new_t = pos * scale + t - pos * scale * zoom
                            // new_t = t + pos * scale * (1 - zoom)
                            self.transform.translation +=
                                zoom_center * self.transform.scaling * (1.0 - zoom);
                            self.transform.scaling *= zoom;
                        }
                    }
                    let transform = transform_viewport(self.transform, screen_viewport);
                    let viewport = transform.inverse() * screen_viewport;

                    ui.ctx().set_transform_layer(layer_id, transform);

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
}

impl PatchTab {
    fn node_view(
        &mut self,
        ctx: &mut crate::Context,
        ui: &mut egui::Ui,
        viewport: Rect,
        viewport_interaction: egui::Response,
    ) {
        let Some(track_id) = self.track_id else {
            unreachable!()
        };
        let Some(Track { patch, .. }) = ctx.state.tracks.get(track_id) else {
            unreachable!();
        };
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
                if ui.button("Note Output").clicked() {
                    node_added = Some(Box::new(
                        cubedaw_workerlib::nodes::NoteOutputNode::new_state(Default::default()),
                    ));
                }
                if ui.button("Track Output").clicked() {
                    node_added = Some(Box::new(
                        cubedaw_workerlib::nodes::TrackOutputNode::new_state(Default::default()),
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
                        ui.visuals().faint_bg_color.gamma_multiply(3.0),
                    );
                }
            }
        }

        let result = self
            .drag_handler
            .handle_snapped(std::convert::identity, |prepared| {
                let handle_node = |ui: &mut egui::Ui,
                                   node_data: &NodeData,
                                   node_id: Option<Id<NodeData>>,
                                   node_ui: &NodeUiState,
                                   tracker: &mut UiStateTracker| {
                    const NODE_MARGIN: f32 = 8.0;

                    let pos = node_ui.pos;

                    let mut child_ui = ui.child_ui_with_id_source(
                        Rect::from_x_y_ranges(pos.x..=pos.x + node_ui.width, pos.y..=f32::INFINITY),
                        egui::Layout::top_down(egui::Align::Min),
                        node_id.unwrap_or(Id::new("currently_held_node")),
                    );

                    let mut frame = egui::Frame::window(child_ui.style());
                    if node_ui.selected {
                        // TODO "stealing" another stroke's color is kinda goofy.
                        // decide if this is okay or find a workaround
                        frame.stroke.color = ui.visuals().widgets.active.fg_stroke.color;
                    }

                    let mut prepared = frame.begin(&mut child_ui);
                    {
                        let mut ui_ctx = CubedawNodeUiContext::new(node_id, track_id, node_data);

                        let node_state = node_data.inner.as_ref();
                        let mut inner_node_ui = node_state.clone();
                        inner_node_ui.ui(&mut prepared.content_ui, &mut ui_ctx);
                        if *inner_node_ui != *node_state
                            && let Some(node_id) = node_id
                        {
                            tracker.add(NodeUiUpdate::new(track_id, node_id, inner_node_ui))
                        }

                        ui_ctx.finish(tracker);

                        if let Some(node_id) = node_id {
                            // only interact if the node is real.
                            // this prevents issues where the user clicks on the node instead of the background by accident
                            let response = prepared.allocate_space(&mut child_ui);
                        }
                    }
                    prepared.paint(&child_ui);
                };
                for (node_id, node_data) in patch.nodes() {
                    let node_ui = patch_ui.nodes.get(node_id).expect("nonexistent node ui");

                    handle_node(ui, node_data, Some(node_id), node_ui, &mut ctx.tracker);
                }

                // .take() is used to avoid doing an obvious unwrap when checking if the node should be placed.
                // if the node isn't placed, the node is put back into currently_held_node at the end.
                if let Some(node_data) = self.currently_held_node.take() {
                    if let Some(hover_pos) = {
                        // TODO hover_pos is broken. https://github.com/emilk/egui/pull/4679
                        // replace this when the version is updated to include that PR

                        // viewport_interaction.hover_pos()

                        if viewport_interaction.hovered()
                            && let Some(mut pos) =
                                viewport_interaction.ctx.input(|i| i.pointer.hover_pos())
                        {
                            if let Some(transform) = viewport_interaction.ctx.memory(|m| {
                                m.layer_transforms
                                    .get(&viewport_interaction.layer_id)
                                    .cloned()
                            }) {
                                pos = transform.inverse() * pos;
                            }
                            Some(pos)
                        } else {
                            None
                        }
                    } {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::AllScroll);
                        handle_node(
                            ui,
                            &node_data,
                            None,
                            &NodeUiState {
                                selected: true,
                                pos: hover_pos,
                                width: 128.0,
                            },
                            &mut ctx.tracker,
                        );
                    }
                    if viewport_interaction.clicked()
                        && let Some(interact_pos) = viewport_interaction.interact_pointer_pos()
                    {
                        // place the node
                        ctx.tracker.add(UiNodeAddOrRemove::addition(
                            Id::arbitrary(),
                            NodeData::new_disconnected(node_data.key, node_data.inner),
                            track_id,
                            NodeUiState {
                                selected: true,
                                pos: interact_pos,
                                width: 128.0, // TODO impl node widths
                            },
                        ))
                    } else {
                        // otherwise, put it back
                        self.currently_held_node = Some(node_data);
                    }
                }
            });

        {
            let should_deselect_everything = result.should_deselect_everything();
            let selection_changes = result.selection_changes();
            if should_deselect_everything {
                // TODO rename these
                for (&track_id2, track_ui) in &ctx.ui_state.tracks {
                    for (&node_id2, node_ui) in &track_ui.patch.nodes {
                        if node_ui.selected
                            && selection_changes.get(&(track_id2, node_id2)).copied() != Some(true)
                        {
                            ctx.tracker
                                .add(UiNodeSelect::new(track_id2, node_id2, false));
                        }
                    }
                }
            } else {
                for (&(track_id, node_id), &selected) in selection_changes {
                    ctx.tracker
                        .add(UiNodeSelect::new(track_id, node_id, selected));
                }
            }
        }
    }
}

struct CubedawNodeUiContext<'a> {
    id: Option<Id<NodeData>>,
    track_id: Id<Track>,
    node_data: &'a NodeData,

    input_counter: usize,
    output_counter: usize,

    tracker: UiStateTracker,
}
impl<'a> CubedawNodeUiContext<'a> {
    pub fn new(id: Option<Id<NodeData>>, track_id: Id<Track>, node_data: &'a NodeData) -> Self {
        Self {
            id,
            track_id,
            node_data,

            input_counter: 0,
            output_counter: 0,

            tracker: UiStateTracker::new(),
        }
    }

    fn finish(self, tracker: &mut UiStateTracker) {
        if let Some(id) = self.id {
            let old_num_inputs = self.node_data.inputs.len();
            if self.input_counter < old_num_inputs {
                let deleted_input_range = self.input_counter..old_num_inputs;
                for (deleted_input, input_index) in self.node_data.inputs
                    [deleted_input_range.clone()]
                .iter()
                .zip(deleted_input_range)
                {
                    // do in reverse order because removing elements one-by-one from the vec is faster if you remove from last to first
                    for &connection in deleted_input.connections.iter().rev() {
                        tracker.add(cubedaw_command::patch::CableAddOrRemove::removal(
                            connection,
                            self.track_id,
                        ));

                        ();
                    }
                    tracker.add(cubedaw_command::node::NodeInputAddOrRemove::removal(
                        id,
                        self.track_id,
                        input_index,
                        deleted_input.value,
                    ));
                }
            }
        }

        tracker.extend(self.tracker);
    }
}

impl cubedaw_lib::NodeUiContext for CubedawNodeUiContext<'_> {
    fn input_ui(&mut self, ui: &mut egui::Ui, name: &str, options: NodeInputUiOptions) {
        let input = self.node_data.inputs.get(self.input_counter);

        let previous_value = match input {
            Some(input) => input.value,
            None => {
                if let Some(id) = self.id {
                    self.tracker
                        .add(cubedaw_command::node::NodeInputAddOrRemove::addition(
                            id,
                            self.track_id,
                            self.input_counter,
                            options.default_value,
                        ));
                }
                options.default_value
            }
        };
        let mut value = previous_value;

        ui.add(DragValue::new(&mut value));

        if previous_value != value
            && let Some(id) = self.id
        {
            self.tracker
                .add(cubedaw_command::node::NodeInputChange::new(
                    id,
                    self.track_id,
                    self.input_counter,
                    previous_value,
                    value,
                ));
        }

        self.input_counter += 1;
    }
    fn output_ui(&mut self, ui: &mut egui::Ui, name: &str) {
        // do NOTHING!!!! TODO
    }
}
