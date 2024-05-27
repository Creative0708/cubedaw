use std::any::Any;

use cubedaw_lib::{Id, State};

use crate::{app::Tab, command::UiStateCommand, EphemeralState, Screen, UiState};

pub struct Context<'a> {
    // State: global data required to render the music; i.e. volumes, notes, etc
    // This can't be mutated directly, but instead done through commands that can be tracked (for the undo system, synchronizing state to workers, etc.)
    pub state: &'a State,

    // Ui State: global data saved and persisted across launches, but not required to render the music; track names, track ordering, etc.
    // This also can't be mutated directly and is tracked.
    pub ui_state: &'a UiState,

    // Ephemeral State: global data not persisted across launches and is not required to render the music; Drag state
    // This can be mutated directly and is not tracked.
    pub ephemeral_state: &'a mut EphemeralState,

    // Tabs: per-tab state persisted across launches; scroll position, zoom, etc.
    // Also mutable directly and not tracked.
    pub tabs: &'a mut Tabs,

    // State tracker to track events that mutate state or ui_state.
    pub tracker: StateTracker,

    dock_events: Vec<DockEvent>,

    time_since_last_frame: f32,
}

impl<'a> Context<'a> {
    pub fn new(
        state: &'a State,
        ui_state: &'a UiState,
        ephemeral_state: &'a mut EphemeralState,
        tabs: &'a mut Tabs,
        time_since_last_frame: f32,
    ) -> Self {
        Self {
            state,
            ui_state,

            ephemeral_state,
            tabs,

            tracker: StateTracker::new(),
            dock_events: Vec::new(),

            time_since_last_frame,
        }
    }

    pub fn duration_since_last_frame(&self) -> f32 {
        self.time_since_last_frame
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

        self.dock_events.push(DockEvent::AddTabToDockState(id));

        let tab = self.tabs.map.entry(id).or_insert(Box::new(tab));

        // TODO any way to safely remove the unreachable here?
        (&mut **tab as &mut dyn Any)
            .downcast_mut()
            .unwrap_or_else(|| unreachable!())
    }

    pub fn queue_tab_removal_from_map(&mut self, id: Id<Box<dyn Screen>>) {
        self.dock_events.push(DockEvent::RemoveTabFromMap(id))
    }

    pub fn get_single_selected_track(&self) -> Option<Id<cubedaw_lib::Track>> {
        let mut single_selected_track = None;
        for &track_id in &self.ui_state.track_list {
            let track = self
                .ui_state
                .tracks
                .get(track_id)
                .expect("ui_state.track_list not synchronized with ui_state.tracks");
            if track.selected {
                if single_selected_track.is_some() {
                    // more than one selected track, give up
                    single_selected_track = None;
                    break;
                } else {
                    single_selected_track = Some(track_id);
                }
            }
        }
        single_selected_track
    }

    pub fn finish(self) -> ContextResult {
        self.ephemeral_state.selection_rect.finish();
        ContextResult {
            dock_events: self.dock_events,
            state_events: self.tracker.finish(),
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
pub enum DockEvent {
    AddTabToDockState(Id<Tab>),
    RemoveTabFromMap(Id<Tab>),
}

#[derive(Default)]
pub struct StateTracker(Vec<Box<dyn UiStateCommand>>);

impl StateTracker {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn add(&mut self, command: impl UiStateCommand) {
        self.0.push(Box::new(command));
    }
    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }
    pub fn take(&mut self) -> StateTracker {
        core::mem::take(self)
    }
    pub fn finish(self) -> Vec<Box<dyn UiStateCommand>> {
        self.0
    }
}

pub struct ContextResult {
    pub dock_events: Vec<DockEvent>,
    pub state_events: Vec<Box<dyn UiStateCommand>>,
}
