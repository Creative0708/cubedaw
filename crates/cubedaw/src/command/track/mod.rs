use cubedaw_command::{track::TrackAddOrRemove, StateCommand};
use cubedaw_lib::{Id, Track};

use crate::state::ui::TrackUiState;

use super::UiStateCommand;
pub struct UiTrackAddOrRemove {
    inner: TrackAddOrRemove,
    ui_data: Option<TrackUiState>,
    // where the track is inserted in the track list
    insertion_pos: u32,
}

impl UiTrackAddOrRemove {
    pub fn addition(
        id: Id<Track>,
        data: Track,
        ui_data: Option<TrackUiState>,
        insertion_pos: u32,
    ) -> Self {
        Self {
            inner: TrackAddOrRemove::addition(id, data),
            ui_data,
            insertion_pos,
        }
    }
    pub fn removal(id: Id<Track>) -> Self {
        Self {
            inner: TrackAddOrRemove::removal(id),
            ui_data: None,
            insertion_pos: 0,
        }
    }

    fn execute_add(&mut self, ui_state: &mut crate::UiState) {
        ui_state
            .tracks
            .insert(self.inner.id(), self.ui_data.take().unwrap_or_default());
        ui_state.track_list.insert(
            (self.insertion_pos as usize).min(ui_state.track_list.len()),
            self.inner.id(),
        );
    }
    fn execute_remove(&mut self, ui_state: &mut crate::UiState) {
        self.ui_data = ui_state.tracks.remove(self.inner.id());
        ui_state.track_list.retain(|&id| id != self.inner.id());
    }
}

impl UiStateCommand for UiTrackAddOrRemove {
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

    fn inner(&mut self) -> Option<&mut dyn StateCommand> {
        Some(&mut self.inner)
    }
}

pub struct UiTrackRename {
    id: Id<Track>,
    data: String,
}

impl UiTrackRename {
    pub fn new(id: Id<Track>, name: String) -> Self {
        Self { id, data: name }
    }

    fn swap_data(&mut self, ui_state: &mut crate::UiState) {
        core::mem::swap(
            &mut ui_state
                .tracks
                .get_mut(self.id)
                .expect("nonexistent track")
                .name,
            &mut self.data,
        );
    }
}

impl UiStateCommand for UiTrackRename {
    fn ui_execute(&mut self, ui_state: &mut crate::UiState) {
        self.swap_data(ui_state);
    }
    fn ui_rollback(&mut self, ui_state: &mut crate::UiState) {
        self.swap_data(ui_state);
    }
}

pub struct UiTrackSelect {
    id: Id<Track>,
    selected: bool,
}

impl UiTrackSelect {
    pub fn new(id: Id<Track>, selected: bool) -> Self {
        Self { id, selected }
    }
}

impl UiStateCommand for UiTrackSelect {
    fn ui_execute(&mut self, ui_state: &mut crate::UiState) {
        if let Some(ui_data) = ui_state.tracks.get_mut(self.id) {
            ui_data.selected = self.selected;
        }
    }
    fn ui_rollback(&mut self, ui_state: &mut crate::UiState) {
        if let Some(ui_data) = ui_state.tracks.get_mut(self.id) {
            ui_data.selected = !self.selected;
        }
    }
}
