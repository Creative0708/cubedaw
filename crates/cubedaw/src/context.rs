use std::{
    any::Any,
    time::{Duration, Instant},
};

use cubedaw_lib::{Id, Range, State};

use crate::{app::Tab, Screen, SelectionRect, UiState};

pub struct Context {
    // State: global data required to render the music; i.e. volumes, notes, etc
    pub state: State,
    // Ui State: global data persisted across launches, but not required to render the music; track names, track ordering, etc.
    pub ui_state: UiState,
    // Tabs: per-tab state persisted across launches; scroll position, zoom, etc.
    pub tabs: Tabs,

    // Everything else: ephemeral state not persisted; selection box state, etc.
    pub selection_rect: SelectionRect,

    pub is_playing: bool,

    instant: Instant,
    duration_of_last_frame: Duration,

    result: ContextResult,
}

impl Context {
    pub fn new(state: State, ui_state: UiState, tabs: Tabs) -> Self {
        Self {
            state,
            ui_state,
            tabs,

            selection_rect: SelectionRect::new(),

            is_playing: false,

            instant: Instant::now(),
            duration_of_last_frame: Duration::ZERO,

            result: ContextResult::new(),
        }
    }

    pub fn result(&mut self) -> &mut ContextResult {
        &mut self.result
    }

    pub fn get_or_create_tab<T: Screen>(&mut self) -> &mut T {
        if self.tabs.has_tab::<T>() {
            return self.tabs.get_tab().unwrap();
        }

        self.create_tab()
    }
    pub fn create_tab<T: Screen>(&mut self) -> &mut T {
        let tab = T::create(self);
        let id = tab.id();

        self.result.dock_queue.push(DockEvent::Create(id));

        self.tabs.map.insert(id, Box::new(tab));

        let tab = &mut **self.tabs.map.get_mut(&id).unwrap();

        (tab as &mut dyn Any).downcast_mut().unwrap()
    }

    pub fn frame_finished(&mut self, ctx: &egui::Context) {
        let elapsed = self.instant.elapsed();
        if self.is_playing {
            // time * bpm * 60.0 = # of beats
            self.state.needle_pos += ((self.instant.elapsed() - self.duration_of_last_frame)
                .as_micros()
                * Range::UNITS_PER_BEAT as u128
                * 60) as f32
                / (self.state.bpm * 1_000_000f32);
            ctx.request_repaint();
        }
        self.duration_of_last_frame = elapsed;

        if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.is_playing = !self.is_playing;
        }
    }
}

#[derive(Default)]
pub struct Tabs {
    pub map: egui::ahash::HashMap<Id<Tab>, Tab>,
}

impl Tabs {
    pub fn get_tabs<T: Screen>(&mut self) -> impl Iterator<Item = &mut T> {
        return self
            .map
            .iter_mut()
            .filter_map(|(_, tab)| (&mut **tab as &mut dyn Any).downcast_mut::<T>());
    }
    pub fn get_tab<T: Screen>(&mut self) -> Option<&mut T> {
        return self
            .map
            .iter_mut()
            .find_map(|(_, tab)| (&mut **tab as &mut dyn Any).downcast_mut::<T>());
    }
    pub fn has_tab<T: Screen>(&self) -> bool {
        return self
            .map
            .iter()
            .any(|(_, tab)| (&**tab as &dyn Any).is::<T>());
    }
}

#[derive(Debug)]
pub struct ContextResult {
    dock_queue: Vec<DockEvent>,
}

impl ContextResult {
    pub fn new() -> Self {
        Self {
            dock_queue: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.dock_queue.clear();
    }

    pub fn apply_dock_changes(&mut self, dock_state: &mut egui_dock::DockState<Id<Tab>>) {
        while let Some(event) = self.dock_queue.pop() {
            event.apply(dock_state);
        }
    }
}

#[derive(Debug)]
pub enum DockEvent {
    Create(Id<Tab>),
}

impl DockEvent {
    fn apply(self, dock_state: &mut egui_dock::DockState<Id<Tab>>) {
        match self {
            Self::Create(tab_id) => {
                let surface = dock_state.main_surface_mut();
                let root_node = surface.root_node_mut().expect("no root node found?");
                if root_node.is_leaf() && root_node.tabs_count() == 0 {
                    root_node.insert_tab(egui_dock::TabIndex(0), tab_id);
                } else {
                    surface.split_left(egui_dock::NodeIndex::root(), 0.4, vec![tab_id]);
                }
            }
        }
    }
}
