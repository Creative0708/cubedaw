use std::any::Any;

use cubedaw_lib::{Id, IdMap, PreciseSongPos, State};

use crate::{
    EphemeralState, Screen, UiState,
    app::Tab,
    command::{IntoUiStateCommand, UiStateCommand, UiStateCommandWrapper},
    registry::NodeRegistry,
};

pub struct Context<'a> {
    /// Global data required to render the music; i.e. volumes, notes, etc
    /// This can't be mutated directly, but instead done through commands that can be tracked (for the undo system, synchronizing state to workers, etc.)
    pub state: &'a State,

    /// Global data saved and persisted across launches, but not required to render the music; track names, track ordering, etc.
    /// This also can't be mutated directly and is only modifiable through commands.
    pub ui_state: &'a UiState,

    /// Global data not persisted across launches and is not required to render the music; Drag state
    /// This can be mutated directly and is not tracked.
    pub ephemeral_state: &'a mut EphemeralState,

    /// Per-tab state not persisted across launches; scroll position, zoom, etc.
    /// Also mutable directly and not tracked.
    pub tabs: &'a mut Tabs,

    /// App-associated node registry. See [`cubedaw_lib::NodeRegistry`] for more information.
    pub node_registry: &'a NodeRegistry,

    /// State tracker to track events that mutate state or ui_state.
    pub tracker: UiStateTracker,

    // TODO: user preferences
    // pub theme/keybinds/whatever: UserPreferences
    focused_tab: Option<Id<Tab>>,

    time_since_last_frame: f32,

    currently_playing_playhead_pos: Option<PreciseSongPos>,
}

impl<'a> Context<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        state: &'a State,
        ui_state: &'a UiState,
        ephemeral_state: &'a mut EphemeralState,
        tabs: &'a mut Tabs,
        node_registry: &'a NodeRegistry,
        focused_tab: Option<Id<Tab>>,
        time_since_last_frame: f32,
        currently_playing_playhead_pos: Option<PreciseSongPos>,
    ) -> Self {
        Self {
            state,
            ui_state,

            ephemeral_state,
            tabs,
            node_registry,

            tracker: UiStateTracker::new(),

            focused_tab,

            time_since_last_frame,

            currently_playing_playhead_pos,
        }
    }

    pub fn duration_since_last_frame(&self) -> f32 {
        self.time_since_last_frame
    }

    pub fn focused_tab(&self) -> Option<Id<Tab>> {
        self.focused_tab
    }

    pub fn is_playing(&self) -> bool {
        self.currently_playing_playhead_pos.is_some()
    }
    pub fn currently_playing_playhead_pos(&self) -> Option<PreciseSongPos> {
        self.currently_playing_playhead_pos
    }
    pub fn playhead_pos(&self) -> PreciseSongPos {
        match self.currently_playing_playhead_pos() {
            Some(pos) => pos,
            None => PreciseSongPos::from_song_pos(self.ui_state.playhead_pos),
        }
    }

    pub fn finish(self) -> ContextResult {
        self.ephemeral_state.selection_rect.finish();
        ContextResult {
            dock_events: core::mem::take(&mut self.tabs.dock_events),
            tracker: self.tracker.finish(),
        }
    }
}

#[derive(Default)]
pub struct Tabs {
    pub map: IdMap<Tab>,
    dock_events: Vec<DockEvent>,
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

    pub fn get_or_create_tab<T: Screen>(
        &mut self,
        state: &cubedaw_lib::State,
        ui_state: &crate::UiState,
    ) -> &mut T {
        if self.has_tab::<T>() {
            return self.get_tab().unwrap();
        }

        self.create_tab(state, ui_state)
    }
    pub fn create_tab<T: Screen>(
        &mut self,
        state: &cubedaw_lib::State,
        ui_state: &crate::UiState,
    ) -> &mut T {
        let tab = T::create(state, ui_state);
        let id = tab.id();

        self.dock_events.push(DockEvent::AddTabToDockState(id));

        let tab = self.map.insert_and_get_mut(id, Box::new(tab));

        (&mut **tab as &mut dyn Any)
            .downcast_mut()
            .unwrap_or_else(|| unreachable!())
    }

    pub fn queue_tab_removal_from_map(&mut self, id: Id<Box<dyn Screen>>) {
        self.dock_events.push(DockEvent::RemoveTabFromMap(id))
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
    delete_last_command: bool,
}

impl UiStateTracker {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            strong: false,
            delete_last_command: false,
        }
    }
    pub fn add<I: UiStateCommand>(&mut self, command: impl IntoUiStateCommand<I>) {
        // dbg!(std::any::type_name_of_val(&command));
        self.strong = true;
        self.add_weak(command)
    }
    pub fn add_weak<I: UiStateCommand>(&mut self, command: impl IntoUiStateCommand<I>) {
        let command = command.into_ui_state_command();

        // dbg!(std::any::type_name_of_val(&command));
        if let Some(last) = self.commands.last_mut() {
            if last.try_merge(&command) {
                return;
            }
        }
        self.commands.push(Box::new(command));
    }
    /// Signals to the app that it should discard the last command. You better make sure you're deleting what you actually want to delete. You also better make sure that the deleted commands are a net no-op; that is, in sequence, they don't change the state in any way.
    pub fn delete_last_command(&mut self) {
        self.delete_last_command = true;
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
            delete_last_command: self.delete_last_command,
        }
    }
    pub fn len(&self) -> usize {
        self.commands.len()
    }
}

pub struct UiStateTrackerResult {
    pub commands: Vec<Box<dyn UiStateCommandWrapper>>,
    pub strong: bool,
    pub delete_last_command: bool,
}
