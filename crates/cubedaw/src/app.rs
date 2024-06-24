use std::sync::Arc;

use cubedaw_lib::Id;
use egui_dock::{DockArea, DockState};

use crate::{
    command::{UiStateCommand, UiStateCommandWrapper},
    context::DockEvent,
    node,
    tab::{patch::PatchTab, pianoroll::PianoRollTab, track::TrackTab},
    Context, Screen,
};

pub struct CubedawApp {
    // see crate::Context for descriptions
    state: cubedaw_lib::State,
    ui_state: crate::UiState,

    ephemeral_state: crate::EphemeralState,
    tabs: crate::context::Tabs,

    node_registry: Arc<cubedaw_workerlib::NodeRegistry>,

    last_frame_time: f64,

    dock_state: egui_dock::DockState<Id<Tab>>,

    undo_stack: Vec<Vec<Box<dyn UiStateCommandWrapper>>>,

    // The index where the next action will be placed.
    // i.e. if the stack is
    // [1, 2, 3]
    // and the user just undid action 3, then undo_index == 2.
    undo_index: usize,

    worker_host: crate::workerhost::WorkerHostHandle,
}

impl CubedawApp {
    pub fn new(_: &eframe::CreationContext) -> Self {
        let mut app = {
            let mut state = cubedaw_lib::State::default();
            let mut ui_state = crate::UiState::default();
            let mut ephemeral_state = crate::EphemeralState::default();

            fn execute(
                mut command: impl UiStateCommand,
                state: &mut cubedaw_lib::State,
                ui_state: &mut crate::UiState,
                ephemeral_state: &mut crate::EphemeralState,
            ) {
                if let Some(inner) = command.inner() {
                    inner.execute(state);
                }
                command.ui_execute(ui_state, ephemeral_state);
            }

            let node_registry = Arc::new({
                let mut registry = cubedaw_workerlib::NodeRegistry::default();
                node::register_cubedaw_nodes(&mut registry);
                registry
            });

            let track_id = Id::arbitrary();

            execute(
                crate::command::track::UiTrackAddOrRemove::addition(
                    track_id,
                    cubedaw_lib::Track::new_empty(cubedaw_lib::Patch::default()),
                    Some(crate::state::ui::TrackUiState {
                        selected: true,
                        ..Default::default()
                    }),
                    0,
                ),
                &mut state,
                &mut ui_state,
                &mut ephemeral_state,
            );

            Self {
                worker_host: {
                    let mut worker_host = crate::workerhost::WorkerHostHandle::new(
                        state.clone(),
                        cubedaw_workerlib::WorkerOptions {
                            node_registry: node_registry.clone(),

                            sample_rate: 44100,
                            buffer_size: 256,
                        },
                    );
                    worker_host
                },

                state,
                ui_state,
                ephemeral_state,
                tabs: Default::default(),

                node_registry,

                dock_state: DockState::new(Vec::new()),

                last_frame_time: f64::NEG_INFINITY,

                undo_stack: Vec::new(),
                undo_index: 0,
            }
        };

        let mut ctx = Context::new(
            &app.state,
            &app.ui_state,
            &mut app.ephemeral_state,
            &mut app.tabs,
            &app.node_registry,
            0.0,
        );

        ctx.create_tab::<PianoRollTab>();
        // ctx.create_tab::<TrackTab>();
        ctx.create_tab::<PatchTab>();

        let result = ctx.finish();
        app.ctx_finished(result);

        app
    }

    fn ctx_finished(&mut self, result: crate::context::ContextResult) {
        for event in result.dock_events {
            match event {
                DockEvent::AddTabToDockState(tab_id) => {
                    let surface = self.dock_state.main_surface_mut();
                    if let Some(root_node) = surface.root_node_mut() {
                        if root_node.is_leaf() && root_node.tabs_count() == 0 {
                            root_node.insert_tab(egui_dock::TabIndex(0), tab_id);
                        } else {
                            surface.split_left(egui_dock::NodeIndex::root(), 0.4, vec![tab_id]);
                        }
                    } else {
                        surface.push_to_first_leaf(tab_id);
                    }
                }
                DockEvent::RemoveTabFromMap(tab_id) => {
                    self.tabs.map.remove(&tab_id);
                }
            }
        }

        let mut tracker_result = result.tracker;
        for event in &mut tracker_result.commands {
            event.ui_execute(&mut self.ui_state, &mut self.ephemeral_state);
            if let Some(inner) = event.inner() {
                inner.execute(&mut self.state);
            }
        }
        if !tracker_result.commands.is_empty() {
            if self.undo_index < self.undo_stack.len() {
                self.undo_stack
                    .resize_with(self.undo_index, || unreachable!());
            }
            if !tracker_result.strong
                && let Some(last) = self.undo_stack.last_mut()
            {
                let mut starting_index = 0;
                if let (Some(last_command), Some(first_command)) =
                    (last.first_mut(), tracker_result.commands.first_mut())
                {
                    if last_command.try_merge(first_command.as_ref()) {
                        starting_index = 1;
                    }
                }
                last.extend(tracker_result.commands.drain(starting_index..));
            } else {
                // if let (Some(last), Some(first)) = (
                //     self.undo_stack.last_mut().and_then(|x| x.last_mut()),
                //     tracker_result.commands.first(),
                // ) {
                //     if last.try_merge(first.as_ref()) {
                //         tracker_result.commands.remove(0);
                //     }
                // }
                if !tracker_result.commands.is_empty() {
                    tracker_result.commands.shrink_to_fit();
                    self.undo_stack.push(tracker_result.commands);
                    self.undo_index += 1;
                }
            }
        }
    }
}

impl eframe::App for CubedawApp {
    fn update(&mut self, egui_ctx: &egui::Context, _egui_frame: &mut eframe::Frame) {
        let time = egui_ctx.input(|i| i.time);
        let frame_duration = ((time - self.last_frame_time) as f32).min(0.1);
        self.last_frame_time = time;

        let mut ctx = Context::new(
            &self.state,
            &self.ui_state,
            &mut self.ephemeral_state,
            &mut self.tabs,
            &self.node_registry,
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
                        ui.close_menu();
                    }
                    if ui.button("Piano Roll").clicked() {
                        ctx.create_tab::<PianoRollTab>();
                        ui.close_menu();
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
        if !result.tracker.commands.is_empty() {
            egui_ctx.request_repaint();
        }
        self.ctx_finished(result);

        if self.ephemeral_state.is_playing {
            // time * bpm * 60.0 = # of beats
            self.ui_state.playhead_pos += (frame_duration
                * (cubedaw_lib::Range::UNITS_PER_BEAT * 60) as f32)
                / (self.state.bpm * 1_000_000f32);
            egui_ctx.request_repaint();
        }

        // global key commands

        // TODO implement configurable keymaps
        if egui_ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space)) {
            self.ephemeral_state.is_playing = !self.ephemeral_state.is_playing;
        }
        if let Some(is_redo) = egui_ctx.input(|i| {
            (i.modifiers.ctrl && i.key_pressed(egui::Key::Z)).then_some(i.modifiers.shift)
        }) {
            if is_redo {
                if let Some(actions_being_redone) = self.undo_stack.get_mut(self.undo_index) {
                    for action in actions_being_redone.iter_mut() {
                        action.ui_execute(&mut self.ui_state, &mut self.ephemeral_state);
                        if let Some(state_action) = action.inner() {
                            state_action.execute(&mut self.state);
                        }
                    }
                    self.undo_index += 1;
                }
            } else if let Some(actions_being_undone) =
                self.undo_stack.get_mut(self.undo_index.wrapping_sub(1))
            {
                // do undo actions in the opposite order
                for action in actions_being_undone.iter_mut().rev() {
                    action.ui_rollback(&mut self.ui_state, &mut self.ephemeral_state);
                    if let Some(state_action) = action.inner() {
                        state_action.rollback(&mut self.state);
                    }
                }
                self.undo_index -= 1;
            }
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
        self.ctx.queue_tab_removal_from_map(*id);
        true
    }
}
