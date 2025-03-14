use cubedaw_command::section::SectionAddOrRemove;
use cubedaw_lib::{Id, IdMap, Section, Track};

use crate::{state::ui::SectionUiState, util::Select};

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

    fn sections<'a>(
        &self,
        ui_state: &'a mut crate::UiState,
    ) -> &'a mut IdMap<Section, SectionUiState> {
        &mut ui_state
            .tracks
            .get_mut(self.inner.track_id())
            .expect("nonexistent track")
            .sections
    }

    fn execute_add(&mut self, ui_state: &mut crate::UiState) {
        self.sections(ui_state)
            .insert(self.inner.id(), self.ui_data.take().unwrap_or_default());
    }
    fn execute_remove(&mut self, ui_state: &mut crate::UiState) {
        self.ui_data = self.sections(ui_state).remove(self.inner.id());
    }
}

impl UiStateCommand for UiSectionAddOrRemove {
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

pub struct UiSectionSelect {
    track_id: Id<Track>,
    id: Id<Section>,
    state: Select,
}

impl UiSectionSelect {
    pub fn new(track_id: Id<Track>, id: Id<Section>, state: Select) -> Self {
        Self {
            track_id,
            id,
            state,
        }
    }

    fn section<'a>(&self, ui_state: &'a mut crate::UiState) -> Option<&'a mut SectionUiState> {
        ui_state
            .tracks
            .get_mut(self.track_id)
            .expect("tried selecting section on nonexistent track")
            .sections
            .get_mut(self.id)
    }
}

impl UiStateCommand for UiSectionSelect {
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = self.section(ui_state) {
            ui_data.selected = self.state;
        }
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = self.section(ui_state) {
            ui_data.selected = !self.state;
        }
    }
}
