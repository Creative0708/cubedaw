use std::marker::PhantomData;

use cubedaw_lib::{math::subrect, State};
use egui::{
    epaint::PathShape, pos2, vec2, Align, Color32, FontData, FontDefinitions, Id, Layout, Rect,
    Rounding, Shape, Stroke, Vec2,
};
use egui_dock::{DockArea, DockState};
use log::info;

use crate::{
    compat::{self, Compat},
    screen::{
        test::TestScreen,
        test2::TestScreen2,
        viewer::{self, CubedawTabViewer},
        Screen, TrackScreen,
    },
    Context,
};

pub struct TestApp {
    dock_state: DockState<viewer::Tab>,
    state: State,
}

impl Default for TestApp {
    fn default() -> Self {
        Self {
            dock_state: DockState::new(vec![
                Box::new(TestScreen::new(Id::new(0))),
                Box::new(TestScreen::new(Id::new(1))),
                Box::new(TestScreen::new(Id::new(2))),
                Box::new(TestScreen2::new(Id::new(3))),
                Box::new(TestScreen2::new(Id::new(4))),
                Box::new(TrackScreen::new(Id::new(5))),
            ]),
            state: State::default(),
        }
    }
}

impl TestApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::resources::style::set_style(&cc.egui_ctx);

        Self::default()
    }
}

impl eframe::App for TestApp {
    fn update(&mut self, egui_ctx: &egui::Context, egui_frame: &mut eframe::Frame) {
        let context = Context {
            state: &mut self.state,
            paused: false,
        };

        let style = egui_ctx.style();

        egui::TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
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
                #[cfg(debug_assertions)]
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::warn_if_debug_build(ui);
                });
            });
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(&style).fill(style.visuals.extreme_bg_color))
            .show(egui_ctx, |ui| {
                DockArea::new(&mut self.dock_state)
                    .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                    .show_inside(ui, &mut CubedawTabViewer::new(context))
            });
    }
}
