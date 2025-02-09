use std::sync::Arc;

use crate::registry::NodeRegistry;
use cpal::traits::HostTrait;
use cubedaw_lib::Id;
use egui_dock::{DockArea, DockState};

use crate::{
    Context, Screen,
    command::{UiStateCommand, UiStateCommandWrapper},
    context::DockEvent,
    node,
};

pub struct CubedawApp {
    // see crate::Context for descriptions
    state: cubedaw_lib::State,
    ui_state: crate::UiState,

    ephemeral_state: crate::EphemeralState,
    tabs: crate::context::Tabs,

    node_registry: Arc<NodeRegistry>,

    last_frame_time: std::time::Instant,

    dock_state: egui_dock::DockState<Id<Tab>>,

    undo_stack: Vec<Vec<Box<dyn UiStateCommandWrapper>>>,

    /// The index where the next action will be placed.
    /// i.e. if the stack is
    /// ```
    /// [1, 2, 3]
    /// ```
    /// and the user just undid action `3`, then `undo_index == 2`.
    undo_index: usize,

    worker_host: crate::workerhost::WorkerHostHandle,
}

impl CubedawApp {
    pub fn new(creation_context: &eframe::CreationContext) -> Self {
        let mut app = {
            let mut state = cubedaw_lib::State::default();
            let mut ui_state = crate::UiState::default();
            ui_state.show_root_track = true;
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
                let mut registry = NodeRegistry::default();
                node::register_cubedaw_nodes(&mut registry);
                registry
            });

            execute(
                crate::command::track::UiTrackAddOrRemove::add_generic_group_track(
                    Id::arbitrary(),
                    None,
                    0,
                    &node_registry,
                ),
                &mut state,
                &mut ui_state,
                &mut ephemeral_state,
            );

            let section_track_id = Id::arbitrary();
            execute(
                crate::command::track::UiTrackAddOrRemove::add_generic_section_track(
                    section_track_id,
                    Some(state.root_track),
                    0,
                    &node_registry,
                ),
                &mut state,
                &mut ui_state,
                &mut ephemeral_state,
            );

            execute(
                crate::command::track::UiTrackSelect::new(section_track_id, true),
                &mut state,
                &mut ui_state,
                &mut ephemeral_state,
            );

            Self {
                worker_host: crate::workerhost::WorkerHostHandle::new(),

                state,
                ui_state,
                ephemeral_state,
                tabs: Default::default(),

                node_registry,

                dock_state: DockState::new(Vec::new()),

                last_frame_time: std::time::Instant::now(),

                undo_stack: Vec::new(),
                undo_index: 0,
            }
        };

        let ctx = Context::new(
            &app.state,
            &app.ui_state,
            &mut app.ephemeral_state,
            &mut app.tabs,
            &app.node_registry,
            None,
            0.0,
            None,
        );

        ctx.tabs
            .create_tab::<crate::tab::pianoroll::PianoRollTab>(ctx.state, ctx.ui_state);
        ctx.tabs
            .create_tab::<crate::tab::track::TrackTab>(ctx.state, ctx.ui_state);
        // ctx.tabs
        //     .create_tab::<crate::tab::patch::PatchTab>(ctx.state, ctx.ui_state);

        let result = ctx.finish();
        app.ctx_finished(result, &creation_context.egui_ctx);

        app
    }

    fn ctx_finished(&mut self, result: crate::context::ContextResult, egui_ctx: &egui::Context) {
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
                    let tab = self
                        .tabs
                        .map
                        .remove(tab_id)
                        .expect("tried to remove nonexistent tab");
                    tab.drop(egui_ctx);
                }
            }
        }

        'handle_tracker: {
            let crate::context::UiStateTrackerResult {
                mut commands,
                strong,
                delete_last_command,
            } = result.tracker;

            if delete_last_command {
                self.undo_index -= 1;
                self.undo_stack.truncate(self.undo_index);
            }

            if commands.is_empty() {
                break 'handle_tracker;
            }

            let mut state_commands = Vec::new();
            for event in &mut commands {
                event.ui_execute(&mut self.ui_state, &mut self.ephemeral_state);
                if let Some(inner) = event.inner() {
                    if self.worker_host.is_init() {
                        state_commands.push(inner.clone());
                    }
                    inner.execute(&mut self.state);
                }
            }
            if self.worker_host.is_init() && !state_commands.is_empty() {
                self.worker_host
                    .send_commands(state_commands.into_boxed_slice(), false);
            }

            if self.undo_index < self.undo_stack.len() {
                self.undo_stack
                    .resize_with(self.undo_index, || unreachable!());
            }
            if !strong && let Some(last) = self.undo_stack.last_mut() {
                let mut starting_index = 0;
                if let (Some(last_command), Some(first_command)) =
                    (last.first_mut(), commands.first_mut())
                {
                    if last_command.try_merge(first_command.as_ref()) {
                        starting_index = 1;
                    }
                }
                last.extend(commands.drain(starting_index..));
            } else if !commands.is_empty() {
                self.undo_stack.push(commands);
                self.undo_index += 1;
            }
        }
    }
}

impl eframe::App for CubedawApp {
    fn update(&mut self, egui_ctx: &egui::Context, _egui_frame: &mut eframe::Frame) {
        let time = std::time::Instant::now();
        let frame_duration = (time - self.last_frame_time).as_secs_f32().min(0.1);
        self.last_frame_time = time;

        self.worker_host.handle_events();

        let ctx =
            Context::new(
                &self.state,
                &self.ui_state,
                &mut self.ephemeral_state,
                &mut self.tabs,
                &self.node_registry,
                self.dock_state.find_active_focused().map(|(_, &mut id)| id),
                frame_duration,
                if let Some(last_playhead_update) = self.worker_host.last_playhead_update() {
                    Some(self.state.add_time_to_position(
                        last_playhead_update.0,
                        time - last_playhead_update.1,
                    ))
                } else {
                    None
                },
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
                        ctx.tabs
                            .create_tab::<crate::tab::track::TrackTab>(ctx.state, ctx.ui_state);
                        ui.close_menu();
                    }
                    if ui.button("Patch Editor").clicked() {
                        ctx.tabs
                            .create_tab::<crate::tab::patch::PatchTab>(ctx.state, ctx.ui_state);
                        ui.close_menu();
                    }
                    if ui.button("Piano Roll").clicked() {
                        ctx.tabs.create_tab::<crate::tab::pianoroll::PianoRollTab>(
                            ctx.state,
                            ctx.ui_state,
                        );
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

        // this is where all the tab rendering actually happens!
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
        self.ctx_finished(result, egui_ctx);

        let now = std::time::Instant::now();

        // global key commands

        // TODO implement configurable keymaps
        if egui_ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Space)) {
            if !self.worker_host.is_init() {
                // TODO change/make configurable/whatever
                self.worker_host.init(
                    self.state.clone(),
                    cubedaw_worker::WorkerOptions::new(self.node_registry.inner().clone()),
                );
                self.worker_host.set_device(Some(
                    cpal::default_host()
                        .default_output_device()
                        .expect("no default output device. sorry!"),
                ));
            }
            if !self.worker_host.is_playing() {
                self.worker_host.reset();
                self.worker_host
                    .start_processing(self.ui_state.playhead_pos);
            } else {
                if let Some(last_playhead_update) = self.worker_host.last_playhead_update() {
                    self.ui_state.playhead_pos = self
                        .state
                        .add_time_to_position(last_playhead_update.0, now - last_playhead_update.1)
                        .round_to_song_pos();
                }
                self.worker_host.stop_processing();
            }
        }

        // undo system
        if egui_ctx.memory(|mem| mem.focused().is_none())
            && let Some(is_redo) = egui_ctx.input(|i| {
                (i.modifiers.ctrl && i.key_pressed(egui::Key::Z)).then_some(i.modifiers.shift)
            })
        {
            let mut state_commands = Vec::new();
            if is_redo {
                if let Some(actions_being_redone) = self.undo_stack.get_mut(self.undo_index) {
                    for action in actions_being_redone.iter_mut() {
                        action.ui_execute(&mut self.ui_state, &mut self.ephemeral_state);
                        if let Some(state_action) = action.inner() {
                            if self.worker_host.is_init() {
                                state_commands.push(state_action.clone());
                            }
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
                        if self.worker_host.is_init() {
                            state_commands.push(state_action.clone());
                        }
                        state_action.rollback(&mut self.state);
                    }
                }
                self.undo_index -= 1;
            }
            if self.worker_host.is_init() {
                self.worker_host
                    .send_commands(state_commands.into_boxed_slice(), !is_redo);
            }
        }

        // final stuff
        if self.worker_host.is_playing() {
            egui_ctx.request_repaint();
        }
    }
}

pub type Tab = Box<dyn Screen>;

pub struct CubedawTabViewer<'a> {
    ctx: Context<'a>,
}

impl<'a> egui_dock::TabViewer for CubedawTabViewer<'a> {
    type Tab = Id<Tab>;

    fn title(&mut self, &mut id: &mut Self::Tab) -> egui::WidgetText {
        let tab = self.ctx.tabs.map.get(id).unwrap();
        tab.title().text().into()
    }

    fn id(&mut self, &mut id: &mut Self::Tab) -> egui::Id {
        let tab = self.ctx.tabs.map.get(id).unwrap();
        egui::Id::new(tab.id())
    }

    fn ui(&mut self, ui: &mut egui::Ui, &mut id: &mut Self::Tab) {
        let mut tab = self.ctx.tabs.map.remove(id).unwrap();
        if let Err(err) = tab.update(&mut self.ctx, ui) {
            todo!("unhandled error in tab ui: {err}");
        }
        self.ctx.ephemeral_state.selection_rect.draw(ui, id);
        self.ctx.tabs.map.insert(tab.id(), tab);
    }

    fn on_close(&mut self, &mut id: &mut Self::Tab) -> bool {
        self.ctx.tabs.queue_tab_removal_from_map(id);
        true
    }
}
