use std::sync::Arc;

// use cubedaw_command::StateCommand as _;
use cubedaw_lib::{Id, NodeData, ResourceKey};
use egui_dock::{DockArea, DockState};

use crate::{
    command::UiStateCommand,
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

    node_registry: Arc<node::NodeRegistry>,

    last_frame_time: f64,

    dock_state: egui_dock::DockState<Id<Tab>>,

    // Vec<T>s are used instead of Box<[T]>s bc they can be "joined" with weak statecommands
    // Vec<(is collapsible, events)>
    undo_stack: Vec<(bool, Vec<Box<dyn UiStateCommand>>)>,
    // The index of where the next action will be placed.
    // i.e. if the stack is
    // [1, 2, 3]
    // and the user just undid action 3, then undo_index == 2.
    undo_index: usize,
}

impl CubedawApp {
    pub fn new(_: &eframe::CreationContext) -> Self {
        let mut app = Self {
            state: cubedaw_lib::State::default(),
            ui_state: Default::default(),
            ephemeral_state: crate::EphemeralState::default(),
            tabs: Default::default(),

            node_registry: Arc::new(node::NodeRegistry::default()),

            dock_state: DockState::new(Vec::new()),

            last_frame_time: f64::NEG_INFINITY,

            undo_stack: Vec::new(),
            undo_index: 0,
        };

        let mut ctx = Context::new(
            &app.state,
            &app.ui_state,
            &mut app.ephemeral_state,
            &mut app.tabs,
            0.0,
        );

        let track_id = Id::arbitrary();
        ctx.tracker
            .add(crate::command::track::UiTrackAddOrRemove::addition(
                track_id,
                cubedaw_lib::Track::new_empty({
                    let mut patch = cubedaw_lib::Patch::default();
                    patch.insert_node(
                    patch
                }),
                Some(crate::state::ui::TrackUiState {
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
            0.0,
        );

        ctx.create_tab::<PianoRollTab>();
        // ctx.create_tab::<TrackTab>();
        ctx.create_tab::<PatchTab>();

        let result = ctx.finish();
        app.ctx_finished(result);

        app
    }

    fn ctx_finished(&mut self, mut result: crate::context::ContextResult) {
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

        let mut is_collapsible = true;
        for event in &mut result.state_events {
            event.ui_execute(&mut self.ui_state);
            if let Some(inner) = event.inner() {
                inner.execute(&mut self.state);
                // events that modify state aren't collapsible
                is_collapsible = false;
            }
        }
        if !result.state_events.is_empty() {
            if self.undo_index < self.undo_stack.len() {
                self.undo_stack
                    .resize_with(self.undo_index, || unreachable!());
            }
            result.state_events.shrink_to_fit();
            if is_collapsible && let Some((true, last)) = self.undo_stack.last_mut() {
                last.extend(result.state_events)
            } else {
                self.undo_stack.push((is_collapsible, result.state_events));
                self.undo_index += 1;
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
        if !result.state_events.is_empty() {
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
        if !egui_ctx.wants_keyboard_input() {
            // TODO implement configurable keymaps
            if egui_ctx.input(|i| i.key_pressed(egui::Key::Space)) {
                self.ephemeral_state.is_playing = !self.ephemeral_state.is_playing;
            }
            if let Some(is_redo) = egui_ctx.input(|i| {
                (i.modifiers.ctrl && i.key_pressed(egui::Key::Z)).then_some(i.modifiers.shift)
            }) {
                if is_redo {
                    if let Some((_, actions_being_redone)) =
                        self.undo_stack.get_mut(self.undo_index)
                    {
                        for action in actions_being_redone.iter_mut() {
                            action.ui_execute(&mut self.ui_state);
                            if let Some(state_action) = action.inner() {
                                state_action.execute(&mut self.state);
                            }
                        }
                        self.undo_index += 1;
                    }
                } else if let Some((_, actions_being_undone)) =
                    self.undo_stack.get_mut(self.undo_index.wrapping_sub(1))
                {
                    // do undo actions in the opposite order
                    for action in actions_being_undone.iter_mut().rev() {
                        action.ui_rollback(&mut self.ui_state);
                        if let Some(state_action) = action.inner() {
                            state_action.rollback(&mut self.state);
                        }
                    }
                    self.undo_index -= 1;
                }
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
