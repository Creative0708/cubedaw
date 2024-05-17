use cubedaw_command::section::SectionAddOrRemove;
use cubedaw_lib::{Id, Section, Track};

use crate::ui_state::SectionUiState;

use super::UiStateCommand;

pub struct UiSectionAddOrRemove {
    inner: SectionAddOrRemove,
    ui_data: Option<SectionUiState>,
}

impl UiSectionAddOrRemove {
    pub fn addition(id: Id<Section>, start_pos: i64, data: Section, track_id: Id<Track>) -> Self {
        Self {
            inner: SectionAddOrRemove::addition(id, start_pos, data, track_id),
            ui_data: None,
        }
    }
    pub fn removal(id: Id<Section>, start_pos: i64, track_id: Id<Track>) -> Self {
        Self {
            inner: SectionAddOrRemove::removal(id, start_pos, track_id),
            ui_data: None,
        }
    }

    fn execute_add(&mut self, ui_state: &mut crate::UiState) {
        ui_state
            .sections
            .set(self.inner.id(), self.ui_data.take().unwrap_or_default());
    }
    fn execute_remove(&mut self, ui_state: &mut crate::UiState) {
        self.ui_data = ui_state.sections.remove(self.inner.id());
    }
}

impl UiStateCommand for UiSectionAddOrRemove {
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

pub struct UiSectionSelect {
    id: Id<Section>,
    selected: bool,
}

impl UiSectionSelect {
    pub fn new(id: Id<Section>, selected: bool) -> Self {
        Self { id, selected }
    }
}

impl UiStateCommand for UiSectionSelect {
    fn ui_execute(&mut self, ui_state: &mut crate::UiState) {
        if let Some(ui_data) = ui_state.sections.get_mut(self.id) {
            ui_data.selected = self.selected;
        }
    }
    fn ui_rollback(&mut self, ui_state: &mut crate::UiState) {
        if let Some(ui_data) = ui_state.sections.get_mut(self.id) {
            ui_data.selected = !self.selected;
        }
    }
}
