
use egui::Id;

use crate::{screen::{handler::{ScreenHandler, SplitDirection}, test::TestScreen, test2::TestScreen2}, compat::{self, Compat}, Context};


pub struct TestApp {
    screen_handler: ScreenHandler,

    compat: Box<dyn Compat>,
}

impl Default for TestApp {
    fn default() -> Self {
        Self {
            screen_handler: {
                let mut screen_handler = ScreenHandler::new(Box::new(TestScreen::new(Id::from("test1"))));
                
                screen_handler.split(screen_handler.root_id, SplitDirection::Vertical, false, Box::new(TestScreen::new(Id::from("test2"))));

                screen_handler.split(Id::from("test2"), SplitDirection::Horizontal, false, Box::new(TestScreen::new(Id::from("test3"))));

                screen_handler.split(Id::from("test3"), SplitDirection::Vertical, false, Box::new(TestScreen2::new(Id::from("test4"))));

                screen_handler
            },
            compat: compat::create_platform_compat(),
        }
    }
}

impl TestApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for TestApp {
    fn update(&mut self, egui_ctx: &egui::Context, frame: &mut eframe::Frame) {

        let context = Context {
            compat: self.compat.as_ref(),
            egui_ctx,
        };

        egui::TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Quit").clicked() {
                        frame.close();
                    }
                    let _ = ui.button("Do nothing");
                    if ui.button("Panic! (this will crash the app)").clicked() {
                        panic!("PANIC!!!!!");
                    };
                });
            });
        });

        let style = egui_ctx.style();

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&style).fill(style.visuals.extreme_bg_color))
            .show(egui_ctx, |ui| { self.screen_handler.update(&context, ui); });
    }
}