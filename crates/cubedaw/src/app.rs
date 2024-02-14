use cubedaw_lib::{Id, IdMap, State};
use egui::{ahash::HashMapExt as _, Align, Layout};
use egui_dock::{DockArea, DockState, SurfaceIndex};

use crate::{
    context::Tabs,
    screen::{
        viewer::{self, CubedawTabViewer},
        PianoRollScreen, TrackScreen,
    },
    Context,
};

pub struct TestApp {
    dock_state: DockState<Id<()>>,
    tabs: IdMap<(), viewer::Tab>,
    state: State,
}

impl Default for TestApp {
    fn default() -> Self {
        let mut tab_vec = Vec::new();
        let mut tabs = IdMap::new();

        let mut add_tab = |tab: viewer::Tab| {
            tab_vec.push(tab.id().transmute());
            tabs.insert(tab.id().transmute(), tab);
        };
        add_tab(Box::new(TrackScreen::new(Id::new(5))));
        add_tab(Box::new(PianoRollScreen::new(Id::new(6), None)));

        let mut dock_state = DockState::new(tab_vec);

        let Some(track_tab) = dock_state.main_surface().find_tab(&Id::new(5)) else {
            unreachable!();
        };

        dock_state.set_active_tab((SurfaceIndex::main(), track_tab.0, track_tab.1));

        Self {
            dock_state,
            tabs,
            state: State::new(),
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
            tabs: Tabs {
                map: &mut self.tabs,
            },
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
