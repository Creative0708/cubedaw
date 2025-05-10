use cubedaw_lib::{Clip, Id, Range, Track};
use cubedaw_worker::command::{ActionType, StateCommand, StateCommandWrapper};

use crate::{state::ui::ClipUiState, util::Select};

use super::UiStateCommand;

#[derive(Clone)]
pub struct ClipMove {
    track_from: Id<Track>,
    track_to: Id<Track>,
    starting_range: Range,
    new_start_pos: i64,
}

impl ClipMove {
    // pub fn same(track_id: Id<Track>, starting_range: Range, new_start_pos: i64) -> Self {
    //     Self {
    //         track_from: track_id,
    //         track_to: track_id,
    //         starting_range,
    //         new_start_pos,
    //     }
    // }
    pub fn new(
        track_from: Id<Track>,
        track_to: Id<Track>,
        starting_range: Range,
        new_start_pos: i64,
    ) -> Self {
        Self {
            track_from,
            track_to,
            starting_range,
            new_start_pos,
        }
    }
}
impl StateCommand for ClipMove {
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionType) {
        let (track_from_id, track_to_id, starting_range, new_start_pos) = match action {
            ActionType::Execute => (
                self.track_from,
                self.track_to,
                self.starting_range,
                self.new_start_pos,
            ),
            ActionType::Rollback => (
                self.track_to,
                self.track_from,
                self.starting_range.with_start_pos(self.new_start_pos),
                self.starting_range.start,
            ),
        };

        let track_from = state.tracks.force_get_mut(track_from_id);
        if track_from_id == track_to_id {
            track_from.move_clip(starting_range, new_start_pos);
        } else {
            let (clip_id, clip) = track_from.remove_clip_from_range(starting_range);

            let track_to = state.tracks.force_get_mut(track_to_id);
            track_to.add_clip(clip_id, new_start_pos, clip);
        }
    }
}

// TODO see TrackAddOrRemove
#[derive(Clone)]
struct NoUiClipAddOrRemove {
    track_id: Id<Track>,
    id: Id<Clip>,
    start_pos: i64,
    data: Option<Clip>,
    is_removal: bool,
}

impl NoUiClipAddOrRemove {
    pub fn addition(id: Id<Clip>, start_pos: i64, data: Clip, track_id: Id<Track>) -> Self {
        Self {
            id,
            start_pos,
            track_id,
            data: Some(data),
            is_removal: false,
        }
    }
    pub fn removal(id: Id<Clip>, start_pos: i64, track_id: Id<Track>) -> Self {
        Self {
            start_pos,
            id,
            track_id,
            data: None,
            is_removal: true,
        }
    }
}

impl StateCommand for NoUiClipAddOrRemove {
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionType) {
        let track = state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to add clip to nonexistent track");

        if self.is_removal ^ action.is_rollback() {
            self.data = Some(track.remove_clip(self.id, self.start_pos));
        } else {
            track.add_clip(
                self.id,
                self.start_pos,
                self.data
                    .take()
                    .expect("execute() called on empty ClipAddOrRemove"),
            );
        }
    }
}

pub struct ClipAddOrRemove {
    inner: NoUiClipAddOrRemove,
    ui_data: Option<ClipUiState>,
}

impl ClipAddOrRemove {
    pub fn addition(id: Id<Clip>, start_pos: i64, data: Clip, track_id: Id<Track>) -> Self {
        Self {
            inner: NoUiClipAddOrRemove::addition(id, start_pos, data, track_id),
            ui_data: None,
        }
    }
    pub fn removal(id: Id<Clip>, start_pos: i64, track_id: Id<Track>) -> Self {
        Self {
            inner: NoUiClipAddOrRemove::removal(id, start_pos, track_id),
            ui_data: None,
        }
    }
}

impl UiStateCommand for ClipAddOrRemove {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
        action: ActionType,
    ) {
        let clips = &mut ui_state
            .tracks
            .get_mut(self.inner.track_id)
            .expect("nonexistent track")
            .clips;
        if self.inner.is_removal ^ action.is_rollback() {
            self.ui_data = clips.remove(self.inner.id);
        } else {
            clips.insert(self.inner.id, self.ui_data.take().unwrap_or_default());
        }
    }

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
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
}

impl UiStateCommand for UiClipSelect {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
        action: ActionType,
    ) {
        let Some(ui_data) = ui_state
            .tracks
            .get_mut(self.track_id)
            .expect("tried selecting clip on nonexistent track")
            .clips
            .get_mut(self.id)
        else {
            return;
        };

        ui_data.select = self.state ^ action.is_rollback();
    }
}
