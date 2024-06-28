use std::any::Any;

use cubedaw_lib::{Id, IdMap, State};
use cubedaw_workerlib::NodeRegistry;

use crate::{
    app::Tab,
    command::{UiStateCommand, UiStateCommandWrapper},
    EphemeralState, Screen, UiState,
};

pub struct Context<'a> {
    // State: global data required to render the music; i.e. volumes, notes, etc
    // This can't be mutated directly, but instead done through commands that can be tracked (for the undo system, synchronizing state to workers, etc.)
    pub state: &'a State,

    // Ui State: global data saved and persisted across launches, but not required to render the music; track names, track ordering, etc.
    // This also can't be mutated directly and is only modifiable through commands.
    pub ui_state: &'a UiState,

    // Ephemeral State: global data not persisted across launches and is not required to render the music; Drag state
    // This can be mutated directly and is not tracked.
    pub ephemeral_state: &'a mut EphemeralState,

    // Tabs: per-tab state persisted across launches; scroll position, zoom, etc.
    // Also mutable directly and not tracked.
    pub tabs: &'a mut Tabs,

    // App-associated node registry. See [`cubedaw_workerlib::NodeRegistry`] for more information.
    pub node_registry: &'a NodeRegistry,

    // State tracker to track events that mutate state or ui_state.
    pub tracker: UiStateTracker,

    dock_events: Vec<DockEvent>,

    focused_tab: Option<Id<Tab>>,

    time_since_last_frame: f32,
}

impl<'a> Context<'a> {
    pub fn new(
        state: &'a State,
        ui_state: &'a UiState,
        ephemeral_state: &'a mut EphemeralState,
        tabs: &'a mut Tabs,
        node_registry: &'a NodeRegistry,
        focused_track: Option<Id<Tab>>,
        time_since_last_frame: f32,
    ) -> Self {
        Self {
            state,
            ui_state,

            ephemeral_state,
            tabs,
            node_registry,

            tracker: UiStateTracker::new(),
            dock_events: Vec::new(),

            focused_tab: focused_track,

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

        let tab = self.tabs.map.insert_and_get_mut(id, Box::new(tab));

        // TODO any way to safely remove the unreachable here?
        (&mut **tab as &mut dyn Any)
            .downcast_mut()
            .unwrap_or_else(|| unreachable!())
    }

    pub fn queue_tab_removal_from_map(&mut self, id: Id<Box<dyn Screen>>) {
        self.dock_events.push(DockEvent::RemoveTabFromMap(id))
    }

    pub fn focused_tab(&self) -> Option<Id<Tab>> {
        self.focused_tab
    }

    pub fn finish(self) -> ContextResult {
        self.ephemeral_state.selection_rect.finish();
        ContextResult {
            dock_events: self.dock_events,
            tracker: self.tracker.finish(),
        }
    }
}

#[derive(Default)]
pub struct Tabs {
    pub map: IdMap<Tab>,
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

pub struct ContextResult {
    pub dock_events: Vec<DockEvent>,
    pub tracker: UiStateTrackerResult,
}

#[derive(Default)]
pub struct UiStateTracker {
    commands: Vec<Box<dyn UiStateCommandWrapper>>,
    strong: bool,
}

impl UiStateTracker {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            strong: false,
        }
    }
    pub fn add(&mut self, command: impl UiStateCommand) {
        // dbg!(std::any::type_name_of_val(&command));
        self.strong = true;
        self.add_weak(command)
    }
    pub fn add_weak(&mut self, command: impl UiStateCommand) {
        // dbg!(std::any::type_name_of_val(&command));
        if let Some(last) = self.commands.last_mut() {
            if last.try_merge(&command) {
                return;
            }
        }
        self.commands.push(Box::new(command));
    }
    pub fn extend(&mut self, other: Self) {
        self.commands.extend(other.commands);
        self.strong |= other.strong;
    }
    pub fn take(&mut self) -> UiStateTracker {
        core::mem::take(self)
    }
    pub fn finish(self) -> UiStateTrackerResult {
        UiStateTrackerResult {
            commands: self.commands,
            strong: self.strong,
        }
    }
    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

pub struct UiStateTrackerResult {
    pub commands: Vec<Box<dyn UiStateCommandWrapper>>,
    pub strong: bool,
}
