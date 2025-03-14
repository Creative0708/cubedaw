use cubedaw_command::note::NoteAddOrRemove;
use cubedaw_lib::{Id, IdMap, Note, Section, Track};

use crate::{state::ui::NoteUiState, util::Select};

use super::UiStateCommand;

pub struct UiNoteAddOrRemove {
    inner: NoteAddOrRemove,
    ui_data: Option<NoteUiState>,
}

impl UiNoteAddOrRemove {
    pub fn addition(
        id: Id<Note>,
        track_id: Id<Track>,
        section_id: Id<Section>,
        start_pos: i64,
        data: Note,
    ) -> Self {
        Self {
            inner: NoteAddOrRemove::addition(id, track_id, section_id, start_pos, data),
            ui_data: None,
        }
    }
    pub fn removal(track_id: Id<Track>, section_id: Id<Section>, id: Id<Note>) -> Self {
        Self {
            inner: NoteAddOrRemove::removal(id, track_id, section_id),
            ui_data: None,
        }
    }

    fn notes<'a>(&self, ui_state: &'a mut crate::UiState) -> &'a mut IdMap<Note, NoteUiState> {
        &mut ui_state
            .tracks
            .get_mut(self.inner.track_id())
            .expect("tried to add note to nonexistent track")
            .sections
            .get_mut(self.inner.section_id())
            .expect("tried to add note to nonexistent section")
            .notes
    }

    fn execute_add(&mut self, ui_state: &mut crate::UiState) {
        self.notes(ui_state)
            .insert(self.inner.id(), self.ui_data.take().unwrap_or_default());
    }
    fn execute_remove(&mut self, ui_state: &mut crate::UiState) {
        self.ui_data = self.notes(ui_state).remove(self.inner.id());
        assert!(self.ui_data.is_some(), "tried to remove nonexistent note");
    }
}

impl UiStateCommand for UiNoteAddOrRemove {
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if self.inner.is_removal() {
            self.execute_remove(ui_state);
        } else {
            self.execute_add(ui_state);
        }
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if self.inner.is_removal() {
            self.execute_add(ui_state);
        } else {
            self.execute_remove(ui_state);
        }
    }
    fn inner(&mut self) -> Option<&mut dyn cubedaw_command::StateCommandWrapper> {
        Some(&mut self.inner)
    }
}

pub struct UiNoteSelect {
    track_id: Id<Track>,
    section_id: Id<Section>,
    id: Id<Note>,
    select: Select,
}

impl UiNoteSelect {
    pub fn new(track_id: Id<Track>, section_id: Id<Section>, id: Id<Note>, select: Select) -> Self {
        Self {
            track_id,
            section_id,
            id,
            select,
        }
    }

    fn notes<'a>(&self, ui_state: &'a mut crate::UiState) -> &'a mut IdMap<Note, NoteUiState> {
        &mut ui_state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to add note to nonexistent track")
            .sections
            .get_mut(self.section_id)
            .expect("tried to add note to nonexistent section")
            .notes
    }
}

impl UiStateCommand for UiNoteSelect {
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = self.notes(ui_state).get_mut(self.id) {
            ui_data.selected = self.select;
        }
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = self.notes(ui_state).get_mut(self.id) {
            ui_data.selected = !self.select;
        }
    }
}
