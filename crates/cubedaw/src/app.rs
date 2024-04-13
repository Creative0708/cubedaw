use cubedaw_lib::{Id, State};
use egui_dock::{DockArea, DockState};
use smallvec::SmallVec;

use crate::{
    tab::{pianoroll::PianoRollTab, track::TrackTab},
    Context, Screen,
};

pub struct CubedawApp {
    ctx: Context,
    dock_state: egui_dock::DockState<Id<Tab>>,
}

impl CubedawApp {
    pub fn new(_: &eframe::CreationContext) -> Self {
        let mut ctx = Context::new(State::tracking(), Default::default(), Default::default());

        let track_id = ctx.state.tracks.create(cubedaw_lib::Track::new());
        ctx.ui_state.track(&ctx.state);
        ctx.state.clear_events();

        ctx.ui_state.tracks.set_mut(track_id, {
            let mut ui_state = crate::state::TrackUiState::default();
            ui_state.name = "Default Track".into();
            ui_state.selected = true;
            ui_state
        });

        ctx.create_tab::<TrackTab>();
        ctx.create_tab::<PianoRollTab>();

        let mut dock_state = DockState::new(Vec::new());
        ctx.result().apply_dock_changes(&mut dock_state);

        Self { ctx, dock_state }
    }
}

impl eframe::App for CubedawApp {
    fn update(&mut self, egui_ctx: &egui::Context, _egui_frame: &mut eframe::Frame) {
        let ctx = &mut self.ctx;
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
                    ctx,
                    deleted_tabs: SmallVec::new(),
                };
                DockArea::new(&mut self.dock_state)
                    .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                    .show_inside(ui, &mut tab_viewer);
                tab_viewer.deleted_tabs
            })
            .inner;
        ctx.frame_finished(egui_ctx);

        for tab in deleted_tabs {
            println!("deleting {tab:?}");
            ctx.tabs.map.remove(&tab);
        }

        ctx.result().apply_dock_changes(&mut self.dock_state);

        ctx.ui_state.track(&ctx.state);
        ctx.state.clear_events();
        ctx.selection_rect.finish();
    }
}

pub type Tab = Box<dyn Screen>;

pub struct CubedawTabViewer<'a> {
    ctx: &'a mut Context,
    deleted_tabs: SmallVec<[Id<Tab>; 2]>,
}

impl<'a> egui_dock::TabViewer for CubedawTabViewer<'a> {
    type Tab = Id<Tab>;

    fn title(&mut self, id: &mut Self::Tab) -> egui::WidgetText {
        let tab = self.ctx.tabs.map.get(id).unwrap();
        tab.title().into()
    }

    fn id(&mut self, id: &mut Self::Tab) -> egui::Id {
        let tab = self.ctx.tabs.map.get(id).unwrap();
        tab.id().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, &mut id: &mut Self::Tab) {
        let mut tab = self.ctx.tabs.map.remove(&id).unwrap();
        tab.update(&mut self.ctx, ui);
        self.ctx.selection_rect.draw(ui, id);
        self.ctx.tabs.map.insert(tab.id(), tab);
    }

    fn on_close(&mut self, id: &mut Self::Tab) -> bool {
        self.deleted_tabs.push(*id);
        true
    }
}
