use cubedaw_lib::math::subrect;
use egui::{
    epaint::PathShape, pos2, vec2, Align, Color32, Id, Layout, Rect, Rounding, Shape, Stroke, Vec2,
};
use log::info;

use crate::{
    compat::{self, Compat},
    screen::{
        handler::{ScreenHandler, SplitAxis, SplitDirection},
        test::TestScreen,
        test2::TestScreen2,
    },
    Context,
};

pub struct TestApp {
    screen_handler: ScreenHandler,
}

impl Default for TestApp {
    fn default() -> Self {
        Self {
            screen_handler: {
                let mut screen_handler =
                    ScreenHandler::new(Box::new(TestScreen::new(Id::from("test1"))));

                screen_handler.split(
                    screen_handler.root_id,
                    SplitDirection::Up,
                    0.5,
                    Box::new(TestScreen::new(Id::from("test2"))),
                );

                screen_handler.split(
                    Id::from("test2"),
                    SplitDirection::Left,
                    0.5,
                    Box::new(TestScreen::new(Id::from("test3"))),
                );

                screen_handler.split(
                    Id::from("test3"),
                    SplitDirection::Up,
                    0.5,
                    Box::new(TestScreen2::new(Id::from("test4"))),
                );

                screen_handler
            },
        }
    }
}

impl TestApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for TestApp {
    fn update(&mut self, egui_ctx: &egui::Context, egui_frame: &mut eframe::Frame) {
        let context = Context {
            egui_frame,
            paused: false,
        };

        let style = egui_ctx.style();

        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::side_top_panel(&style).inner_margin(8.0))
            .show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.menu_button("File", |ui| {
                        #[cfg(not(target_arch = "wasm32"))]
                        if ui.button("Quit").clicked() {
                            egui_frame.close();
                        }
                        let _ = ui.button("Do nothing");
                        if ui.button("Panic! (this will crash the app)").clicked() {
                            panic!("PANIC!!!!!");
                        };
                    });
                    ui.add_space(ui.max_rect().center().x - ui.cursor().left());

                    let play_button =
                        crate::widget::PainterButton::new(|painter, _selected, visuals| {
                            let clip_rect = painter.clip_rect();
                            let icon_color = visuals.text_color();

                            if context.paused {
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
                    if ui.add(play_button).clicked() {}

                    #[cfg(debug_assertions)]
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        egui::warn_if_debug_build(ui);
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&style).fill(style.visuals.extreme_bg_color))
            .show(egui_ctx, |ui| {
                self.screen_handler.update(&context, ui);
            });
    }
}
