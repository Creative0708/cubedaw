use std::time;

// use cubedaw_command::StateCommand as _;
use cubedaw_lib::Id;
use egui_dock::{DockArea, DockState};

use crate::{
    command::UiStateCommand,
    tab::{pianoroll::PianoRollTab, track::TrackTab},
    Context, Screen,
};

pub struct CubedawApp {
    // see crate::Context for descriptions
    state: cubedaw_lib::State,
    ui_state: crate::UiState,

    ephemeral_state: crate::EphemeralState,
    tabs: crate::context::Tabs,

    last_frame_instant: time::Instant,

    dock_state: egui_dock::DockState<Id<Tab>>,

    undo_stack: Vec<Box<[Box<dyn UiStateCommand>]>>,
}

impl CubedawApp {
    pub fn new(_: &eframe::CreationContext) -> Self {
        let mut app = Self {
            state: cubedaw_lib::State::new(),
            ui_state: Default::default(),
            ephemeral_state: crate::EphemeralState::new(),
            tabs: Default::default(),
            dock_state: DockState::new(Vec::new()),

            last_frame_instant: time::Instant::now(),

            undo_stack: Vec::new(),
        };

        let mut ctx = Context::new(
            &app.state,
            &app.ui_state,
            &mut app.ephemeral_state,
            &mut app.tabs,
            time::Duration::ZERO,
        );

        let track_id = Id::arbitrary();
        ctx.tracker
            .add(crate::command::track::UiTrackAddOrRemove::addition(
                track_id,
                cubedaw_lib::Track::new(),
                Some(crate::ui_state::TrackUiState {
                    name: "Default Track".into(),
                    selected: true,
                }),
                ctx.ui_state.track_list.len() as u32,
            ));

        let result = ctx.finish();
        app.ctx_finished(result);

        let mut ctx = Context::new(
            &app.state,
            &app.ui_state,
            &mut app.ephemeral_state,
            &mut app.tabs,
            time::Duration::ZERO,
        );

        ctx.create_tab::<PianoRollTab>();
        ctx.create_tab::<TrackTab>();

        let result = ctx.finish();
        app.ctx_finished(result);

        app
    }

    fn ctx_finished(&mut self, mut result: crate::context::ContextResult) {
        for event in &mut result.state_events {
            event.ui_execute(&mut self.ui_state);
            if let Some(inner) = event.inner() {
                inner.execute(&mut self.state);
            }
        }
        for event in result.dock_events {
            event.apply(&mut self.dock_state);
        }
        self.undo_stack.push(result.state_events.into_boxed_slice());
    }
}

impl eframe::App for CubedawApp {
    fn update(&mut self, egui_ctx: &egui::Context, _egui_frame: &mut eframe::Frame) {
        let now = time::Instant::now();
        let frame_duration = now.duration_since(self.last_frame_instant);
        self.last_frame_instant = now;

        let mut ctx = Context::new(
            &self.state,
            &self.ui_state,
            &mut self.ephemeral_state,
            &mut self.tabs,
            frame_duration,
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
        let dock_state_borrow = &mut self.dock_state;

        // egui takes an owned `impl FnOnce() -> R`, so we're forced to either do these
        // goofy moving-in-and-out shenanigans or have a compiler error because rust
        // thinks that the provided FnOnce can outlive this function... aaaaaaahhhhhhhhhhhh
        let ctx = egui::CentralPanel::default()
            // .frame(egui::Frame::central_panel(&style).fill(style.visuals.extreme_bg_color))
            .show(egui_ctx, move |ui| {
                let mut tab_viewer = CubedawTabViewer { ctx };
                DockArea::new(dock_state_borrow)
                    .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                    .show_inside(ui, &mut tab_viewer);
                tab_viewer.ctx
            })
            .inner;

        let result = ctx.finish();
        self.ctx_finished(result);

        if self.ephemeral_state.is_playing {
            // time * bpm * 60.0 = # of beats
            self.ui_state.playhead_pos += (frame_duration.as_micros()
                * cubedaw_lib::Range::UNITS_PER_BEAT as u128
                * 60) as f32
                / (self.state.bpm * 1_000_000f32);
            egui_ctx.request_repaint();
        }

        if !egui_ctx.wants_keyboard_input() && egui_ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.ephemeral_state.is_playing = !self.ephemeral_state.is_playing;
        }
    }
}

pub type Tab = Box<dyn Screen>;

pub struct CubedawTabViewer<'a> {
    ctx: Context<'a>,
}

impl<'a> egui_dock::TabViewer for CubedawTabViewer<'a> {
    type Tab = Id<Tab>;

    fn title(&mut self, id: &mut Self::Tab) -> egui::WidgetText {
        let tab = self.ctx.tabs.map.get(id).unwrap();
        tab.title()
    }

    fn id(&mut self, id: &mut Self::Tab) -> egui::Id {
        let tab = self.ctx.tabs.map.get(id).unwrap();
        tab.id().into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, &mut id: &mut Self::Tab) {
        let mut tab = self.ctx.tabs.map.remove(&id).unwrap();
        tab.update(&mut self.ctx, ui);
        self.ctx.ephemeral_state.selection_rect.draw(ui, id);
        self.ctx.tabs.map.insert(tab.id(), tab);
    }

    fn on_close(&mut self, id: &mut Self::Tab) -> bool {
        println!("deleting {id:?}");
        self.ctx.tabs.map.remove(id);
        true
    }
}
