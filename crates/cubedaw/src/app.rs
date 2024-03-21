use cubedaw_lib::{Id, State};
use egui::ahash::{HashMap, HashMapExt};
use egui_dock::{DockArea, DockState};

use crate::{context::Tabs, tab::pianoroll::PianoRollTab, Context, Screen};

pub struct CubedawApp {
    dock_state: egui_dock::DockState<Id<Tab>>,
    tabs: HashMap<Id<Tab>, Tab>,

    state: State,
}

impl CubedawApp {
    pub fn new(_: &eframe::CreationContext) -> Self {
        let mut tabs = HashMap::new();

        let mut tab_obj_vec = Vec::new();

        let mut state = Default::default();

        let mut ctx = Context::new(
            &mut state,
            Tabs { map: &mut tabs },
            // paused: false,
        );

        tab_obj_vec.push(Box::new(PianoRollTab::create(&mut ctx)));

        let mut tab_vec = Vec::new();

        for tab in tab_obj_vec {
            tab_vec.push(tab.id());
            tabs.insert(tab.id(), tab);
        }

        let dock_state = DockState::new(tab_vec);

        Self {
            dock_state,
            tabs,
            state,
        }
    }
}

impl eframe::App for CubedawApp {
    fn update(&mut self, egui_ctx: &egui::Context, _egui_frame: &mut eframe::Frame) {
        let context = Context::new(
            &mut self.state,
            Tabs {
                map: &mut self.tabs,
            },
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
                #[cfg(debug_assertions)]
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::warn_if_debug_build(ui);
                });
            });
        });

        egui::CentralPanel::default()
            // .frame(egui::Frame::central_panel(&style).fill(style.visuals.extreme_bg_color))
            .show(egui_ctx, |ui| {
                DockArea::new(&mut self.dock_state)
                    .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                    .show_inside(ui, &mut CubedawTabViewer::new(context))
            });
    }
}

pub type Tab = Box<dyn Screen>;

pub struct CubedawTabViewer<'a> {
    ctx: Context<'a>,
}

impl<'a> CubedawTabViewer<'a> {
    pub fn new(ctx: Context<'a>) -> Self {
        Self { ctx }
    }
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
}
