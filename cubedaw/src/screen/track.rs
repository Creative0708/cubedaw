use cubedaw_lib::math::subrect;
use egui::{
    epaint::PathShape, pos2, vec2, CentralPanel, Id, Margin, Rect, Rounding, Shape, Stroke,
    TopBottomPanel, WidgetText,
};

use super::Screen;

pub struct TrackScreen {
    id: Id,
}

impl TrackScreen {
    pub fn new(id: Id) -> Self {
        Self { id }
    }
}

impl Screen for TrackScreen {
    fn id(&self) -> egui::Id {
        self.id
    }

    fn title(&self) -> WidgetText {
        "Track View".into()
    }

    fn update(&mut self, ctx: &mut crate::Context, ui: &mut egui::Ui) {
        TopBottomPanel::top(self.id.with("top_menu"))
            .frame(
                egui::Frame::side_top_panel(ui.style()).inner_margin(Margin {
                    left: 0.0,
                    right: 0.0,
                    top: 0.0,
                    bottom: 8.0,
                }),
            )
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label("Hmmmm...");
                    ui.label("Anyone there?");
                    ui.label("Oh well...");

                    let play_button =
                        crate::widget::PainterButton::new(|painter, _selected, visuals| {
                            let clip_rect = painter.clip_rect();
                            let icon_color = visuals.text_color();

                            if ctx.paused {
                                // Play button
                                painter.add(Shape::Path(PathShape::convex_polygon(
                                    vec![
                                        clip_rect.lerp_inside(vec2(0.28, 0.21)),
                                        clip_rect.lerp_inside(vec2(0.8, 0.5)),
                                        clip_rect.lerp_inside(vec2(0.28, 0.79)),
                                    ],
                                    icon_color,
                                    Stroke::NONE,
                                )));
                            } else {
                                // Pause button
                                painter.rect_filled(
                                    subrect(
                                        Rect {
                                            min: pos2(0.24, 0.2),
                                            max: pos2(0.4, 0.8),
                                        },
                                        clip_rect,
                                    ),
                                    Rounding::ZERO,
                                    icon_color,
                                );
                                painter.rect_filled(
                                    subrect(
                                        Rect {
                                            min: pos2(0.6, 0.2),
                                            max: pos2(0.76, 0.8),
                                        },
                                        clip_rect,
                                    ),
                                    Rounding::ZERO,
                                    icon_color,
                                );
                                PathShape::convex_polygon(
                                    vec![
                                        clip_rect.lerp_inside(vec2(0.28, 0.21)),
                                        clip_rect.lerp_inside(vec2(0.8, 0.5)),
                                        clip_rect.lerp_inside(vec2(0.28, 0.79)),
                                    ],
                                    icon_color,
                                    Stroke::NONE,
                                );
                            };
                        });
                    if ui.add(play_button).clicked() {
                        ctx.paused = !ctx.paused;
                    }
                });
            });
        CentralPanel::default().show_inside(ui, |ui| {
            ui.label("I'm here!");
            ui.label("Hello!");
            ui.label("Can you hear me???");
        });
    }
}
