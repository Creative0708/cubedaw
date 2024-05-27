use cubedaw_command::node::NodeUiUpdate;
use cubedaw_lib::{Id, NodeStateWrapper, Track};
use egui::{emath::TSTransform, pos2, Rect, Vec2};

pub struct PatchTab {
    id: Id<crate::app::Tab>,

    track_id: Option<Id<Track>>,

    transform: TSTransform,
}

fn transform(transform: TSTransform, viewport: Rect) -> TSTransform {
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

            track_id: ctx.get_single_selected_track(),

            transform: TSTransform::IDENTITY,
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
                let layer_id = egui::LayerId::new(ui.layer_id().order, self.id.into());
                let screen_viewport = ui.max_rect();
                let viewport_interaction = ui.interact(
                    screen_viewport,
                    layer_id.id.with("patch_move"),
                    egui::Sense::click_and_drag(),
                );
                if let Some(hover_pos) = viewport_interaction.hover_pos() {
                    let (scroll_delta, zoom) =
                        ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta()));
                    if scroll_delta != Vec2::ZERO {
                        self.transform.translation += scroll_delta * self.transform.scaling;
                    }
                    if zoom != 1.0 {
                        let zoom_center = (self.transform.inverse()
                            * (hover_pos - screen_viewport.center()).to_pos2())
                        .to_vec2();

                        // the zoom center should stay at the same location after the transform
                        // pos * s + t = pos * (s * zoom) + new_t
                        // new_t = pos * s + t - pos * s * zoom
                        // new_t = t + (pos * s) * (1 - zoom)
                        // new_t = t + pos * (s * (1 - zoom))
                        self.transform.translation +=
                            zoom_center * (self.transform.scaling * (1.0 - zoom));
                        self.transform.scaling *= zoom;
                    }
                }
                let transform = transform(self.transform, screen_viewport);
                let viewport = transform.inverse() * screen_viewport;
                ui.with_layer_id(layer_id, |ui| {
                    let mut viewport_interaction = viewport_interaction;
                    viewport_interaction.layer_id = layer_id;
                    ui.ctx().set_transform_layer(layer_id, transform);
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

        ui.set_clip_rect(viewport);
        let painter = ui.painter();
        painter.rect_filled(
            viewport,
            egui::Rounding::ZERO,
            ui.visuals().extreme_bg_color,
        );

        const DOT_SPACING: f32 = 16.0;

        // don't draw too many circles
        if viewport.width().max(viewport.height()) < 480.0 {
            for x in (viewport.left() / DOT_SPACING).ceil() as i32.. {
                if x as f32 * DOT_SPACING > viewport.right() {
                    break;
                }
                for y in (viewport.top() / DOT_SPACING).ceil() as i32.. {
                    if y as f32 * DOT_SPACING > viewport.bottom() {
                        break;
                    }

                    painter.circle_filled(
                        pos2(x as f32 * DOT_SPACING, y as f32 * DOT_SPACING),
                        1.5,
                        ui.visuals().faint_bg_color,
                    );
                }
            }
        }

        for node_id in patch.nodes() {
            let node = ctx.state.node_datas.get(node_id).expect("nonexistent node");
            let node_ui = ctx
                .ui_state
                .nodes
                .get(node_id)
                .expect("nonexistent node ui");

            const NOTE_MARGIN: f32 = 8.0;

            let frame = egui::Frame::window(ui.style());

            let prepared = frame.begin(ui);
            {
                let pos = node_ui.pos;
                let mut child_ui = ui.child_ui_with_id_source(
                    Rect::from_x_y_ranges(pos.x..=pos.x + node_ui.width, pos.y..=f32::INFINITY),
                    egui::Layout::top_down(egui::Align::Min),
                    node_id,
                );

                let mut ui_ctx = CubedawNodeUiContext {};

                let mut inner_node_ui = node.inner.clone();
                inner_node_ui.ui(&mut child_ui, &mut ui_ctx);
                if !inner_node_ui.eq(&*node.inner) {
                    ctx.tracker.add(NodeUiUpdate::new(node_id, inner_node_ui))
                }
            }
            prepared.paint(ui);
        }
    }
}

struct CubedawNodeUiContext {}

impl cubedaw_lib::NodeUiContext for CubedawNodeUiContext {
    fn input_ui(&mut self, ui: &mut egui::Ui, name: &str) {}
    fn output_ui(&mut self, ui: &mut egui::Ui, name: &str) {}
}
