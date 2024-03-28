use cubedaw_lib::{Id, State};
use egui::ahash::{HashMap, HashMapExt};
use egui_dock::{DockArea, DockState};
use smallvec::SmallVec;

use crate::{
    context::{ContextResult, Tabs},
    tab::{pianoroll::PianoRollTab, track::TrackTab},
    Context, Screen, UiState,
};

pub struct CubedawApp {
    dock_state: egui_dock::DockState<Id<Tab>>,
    tabs: HashMap<Id<Tab>, Tab>,

    state: State,
    ui_state: UiState,
}

impl CubedawApp {
    fn with_ctx<F: FnOnce(&mut Context)>(&mut self, f: F) {
        let mut result = ContextResult::new();
        let mut ctx = Context::new(
            &mut self.state,
            &mut self.ui_state,
            Tabs {
                map: &mut self.tabs,
            },
            &mut result,
            // paused: false,
        );

        f(&mut ctx);

        result.apply_dock_changes(&mut self.dock_state);
    }

    pub fn new(_: &eframe::CreationContext) -> Self {
        let mut s = Self {
            dock_state: DockState::new(Vec::new()),
            tabs: HashMap::new(),
            state: State::tracking(),
            ui_state: Default::default(),
        };

        s.with_ctx(|ctx| {
            ctx.create_tab::<TrackTab>();
            ctx.create_tab::<PianoRollTab>();
        });

        s
    }
}

impl eframe::App for CubedawApp {
    fn update(&mut self, egui_ctx: &egui::Context, _egui_frame: &mut eframe::Frame) {
        let mut result = ContextResult::new();
        let mut ctx = Context::new(
            &mut self.state,
            &mut self.ui_state,
            Tabs {
                map: &mut self.tabs,
            },
            &mut result,
            // paused: false,
        );

        egui::TopBottomPanel::top("top_panel").show(egui_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Quit").clicked() {
                        egui_ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    let _ = ui.button("Do nothing");
                    if ui.button("Panic! (this will crash the app)").clicked() {
                        panic!("PANIC!!!!!");
                    };
                });
                ui.menu_button("Window", |ui| {
                    if ui.button("Tracks").clicked() {
                        ctx.create_tab::<TrackTab>();
                    }
                    if ui.button("Piano Roll").clicked() {
                        ctx.create_tab::<PianoRollTab>();
                    }
                });
                #[cfg(debug_assertions)]
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::warn_if_debug_build(ui);
                });
            });
        });
        let deleted_tabs = egui::CentralPanel::default()
            // .frame(egui::Frame::central_panel(&style).fill(style.visuals.extreme_bg_color))
            .show(egui_ctx, |ui| {
                let mut tab_viewer = CubedawTabViewer {
                    ctx: &mut ctx,
                    deleted_tabs: SmallVec::new(),
                };
                DockArea::new(&mut self.dock_state)
                    .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                    .show_inside(ui, &mut tab_viewer);
                tab_viewer.deleted_tabs
            })
            .inner;

        for tab in deleted_tabs {
            println!("deleting {tab:?}");
            self.tabs.remove(&tab);
        }

        result.apply_dock_changes(&mut self.dock_state);

        self.ui_state.track(&self.state);
    }
}

pub type Tab = Box<dyn Screen>;

pub struct CubedawTabViewer<'a> {
    ctx: &'a mut Context<'a>,
    deleted_tabs: SmallVec<[Id<Tab>; 2]>,
}

impl<'a> egui_dock::TabViewer for CubedawTabViewer<'a> {
    type Tab = Id<Tab>;

    fn title(&mut self, id: &mut Self::Tab) -> egui::WidgetText {
        let tab = self.ctx.tabs.map.get_mut(id).unwrap();
        tab.title().into()
    }

    fn id(&mut self, id: &mut Self::Tab) -> egui::Id {
        let tab = self.ctx.tabs.map.get_mut(id).unwrap();
        tab.id().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, id: &mut Self::Tab) {
        let mut tab = self.ctx.tabs.map.remove(id).unwrap();
        tab.update(&mut self.ctx, ui);
        self.ctx.tabs.map.insert(tab.id(), tab);
    }

    fn on_close(&mut self, id: &mut Self::Tab) -> bool {
        self.deleted_tabs.push(*id);
        true
    }
}
