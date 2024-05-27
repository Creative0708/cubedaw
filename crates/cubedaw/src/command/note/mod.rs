use cubedaw_command::note::NoteAddOrRemove;
use cubedaw_lib::{Id, Note, Section};

use crate::state::ui::NoteUiState;

use super::UiStateCommand;

pub struct UiNoteAddOrRemove {
    inner: NoteAddOrRemove,
    ui_data: Option<NoteUiState>,
}

impl UiNoteAddOrRemove {
    pub fn addition(id: Id<Note>, start_pos: i64, data: Note, section_id: Id<Section>) -> Self {
        Self {
            inner: NoteAddOrRemove::addition(id, start_pos, data, section_id),
            ui_data: None,
        }
    }
    pub fn removal(id: Id<Note>, start_pos: i64, section_id: Id<Section>) -> Self {
        Self {
            inner: NoteAddOrRemove::removal(id, start_pos, section_id),
            ui_data: None,
        }
    }

    fn execute_add(&mut self, ui_state: &mut crate::UiState) {
        ui_state
            .notes
            .insert(self.inner.id(), self.ui_data.take().unwrap_or_default());
    }
    fn execute_remove(&mut self, ui_state: &mut crate::UiState) {
        self.ui_data = ui_state.notes.remove(self.inner.id());
    }
}

impl UiStateCommand for UiNoteAddOrRemove {
    fn ui_execute(&mut self, ui_state: &mut crate::UiState) {
        if self.inner.is_removal() {
            self.execute_remove(ui_state);
        } else {
            self.execute_add(ui_state);
        }
    }
    fn ui_rollback(&mut self, ui_state: &mut crate::UiState) {
        if self.inner.is_removal() {
            self.execute_add(ui_state);
        } else {
            self.execute_remove(ui_state);
        }
    }
    fn inner(&mut self) -> Option<&mut dyn cubedaw_command::StateCommand> {
        Some(&mut self.inner)
    }
}

pub struct UiNoteSelect {
    id: Id<Note>,
    selected: bool,
}

impl UiNoteSelect {
    pub fn new(id: Id<Note>, selected: bool) -> Self {
        Self { id, selected }
    }
}

impl UiStateCommand for UiNoteSelect {
    fn ui_execute(&mut self, ui_state: &mut crate::UiState) {
        if let Some(ui_data) = ui_state.notes.get_mut(self.id) {
            ui_data.selected = self.selected;
        }
    }
    fn ui_rollback(&mut self, ui_state: &mut crate::UiState) {
        if let Some(ui_data) = ui_state.notes.get_mut(self.id) {
            ui_data.selected = !self.selected;
        }
    }
}
