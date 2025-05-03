use cubedaw_command::clip::ClipAddOrRemove;
use cubedaw_lib::{Clip, Id, IdMap, Track};

use crate::{state::ui::ClipUiState, util::Select};

use super::UiStateCommand;

pub struct UiClipAddOrRemove {
    inner: ClipAddOrRemove,
    ui_data: Option<ClipUiState>,
}

impl UiClipAddOrRemove {
    pub fn addition(id: Id<Clip>, start_pos: i64, data: Clip, track_id: Id<Track>) -> Self {
        Self {
            inner: ClipAddOrRemove::addition(id, start_pos, data, track_id),
            ui_data: None,
        }
    }
    pub fn removal(id: Id<Clip>, start_pos: i64, track_id: Id<Track>) -> Self {
        Self {
            inner: ClipAddOrRemove::removal(id, start_pos, track_id),
            ui_data: None,
        }
    }

    fn clips<'a>(&self, ui_state: &'a mut crate::UiState) -> &'a mut IdMap<Clip, ClipUiState> {
        &mut ui_state
            .tracks
            .get_mut(self.inner.track_id())
            .expect("nonexistent track")
            .clips
    }

    fn execute_add(&mut self, ui_state: &mut crate::UiState) {
        self.clips(ui_state)
            .insert(self.inner.id(), self.ui_data.take().unwrap_or_default());
    }
    fn execute_remove(&mut self, ui_state: &mut crate::UiState) {
        self.ui_data = self.clips(ui_state).remove(self.inner.id());
    }
}

impl UiStateCommand for UiClipAddOrRemove {
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

pub struct UiClipSelect {
    track_id: Id<Track>,
    id: Id<Clip>,
    state: Select,
}

impl UiClipSelect {
    pub fn new(track_id: Id<Track>, id: Id<Clip>, state: Select) -> Self {
        Self {
            track_id,
            id,
            state,
        }
    }

    fn clip<'a>(&self, ui_state: &'a mut crate::UiState) -> Option<&'a mut ClipUiState> {
        ui_state
            .tracks
            .get_mut(self.track_id)
            .expect("tried selecting clip on nonexistent track")
            .clips
            .get_mut(self.id)
    }
}

impl UiStateCommand for UiClipSelect {
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = self.clip(ui_state) {
            ui_data.select = self.state;
        }
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = self.clip(ui_state) {
            ui_data.select = !self.state;
        }
    }
}
