use cubedaw_lib::{Id, Track};
use egui::{pos2, Rect};

pub struct PatchTab {
    id: Id<crate::app::Tab>,

    track_id: Option<Id<Track>>,

    // Zoom. If scale == 2.0, everything is twice as large. If scale == 0.5, everything is half as large, etc.
    // TODO actually implement zooming since egui doesn't support transformations
    scale: f32,
}

impl crate::Screen for PatchTab {
    fn create(ctx: &mut crate::Context) -> Self
    where
        Self: Sized,
    {
        Self {
            id: Id::arbitrary(),

            track_id: ctx.get_single_selected_track(),

            scale: 1.0,
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
                egui::ScrollArea::both().show_viewport(ui, |ui, viewport| {
                    self.inner(ctx, ui, viewport);
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
    fn inner(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui, viewport: Rect) {
        let Some(track_id) = self.track_id else {
            unreachable!()
        };
        let patch = &mut ctx.state.tracks.get_mut(track_id).patch;

        let top_left = ui.max_rect().left_top();
        let screen_rect = viewport.translate(top_left.to_vec2());

        let painter = ui.painter_at(screen_rect);
        painter.rect_filled(
            screen_rect,
            egui::Rounding::ZERO,
            ui.visuals().extreme_bg_color,
        );

        const DOT_SPACING: f32 = 16.0;

        for x in (viewport.left() / DOT_SPACING).ceil() as i32.. {
            if x as f32 * DOT_SPACING > viewport.right() {
                break;
            }
            for y in (viewport.top() / DOT_SPACING).ceil() as i32.. {
                if y as f32 * DOT_SPACING > viewport.bottom() {
                    break;
                }

                painter.circle_filled(
                    pos2(x as f32 * DOT_SPACING, y as f32 * DOT_SPACING) + top_left.to_vec2(),
                    1.5,
                    ui.visuals().faint_bg_color,
                );
            }
        }

        for node_id in patch.nodes() {
            let node = ctx.state.nodes.get_mut(node_id);
        }
    }
}
