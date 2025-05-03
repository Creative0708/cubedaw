use cubedaw_lib::{Clip, Id, Range, Track};

use crate::StateCommand;

#[derive(Clone)]
pub struct ClipMove {
    track_from: Id<Track>,
    track_to: Id<Track>,
    starting_range: Range,
    new_start_pos: i64,
}

impl ClipMove {
    pub fn same(track_id: Id<Track>, starting_range: Range, new_start_pos: i64) -> Self {
        Self {
            track_from: track_id,
            track_to: track_id,
            starting_range,
            new_start_pos,
        }
    }
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

fn move_between(
    state: &mut cubedaw_lib::State,
    track_from_id: Id<Track>,
    track_to_id: Id<Track>,
    starting_range: Range,
    new_start_pos: i64,
) {
    let track_from = state.tracks.force_get_mut(track_from_id);
    if track_from_id == track_to_id {
        track_from.move_clip(starting_range, new_start_pos);
    } else {
        let (clip_id, clip) = track_from.remove_clip_from_range(starting_range);

        let track_to = state.tracks.force_get_mut(track_to_id);
        track_to.add_clip(clip_id, new_start_pos, clip);
    }
}

impl StateCommand for ClipMove {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        move_between(
            state,
            self.track_from,
            self.track_to,
            self.starting_range,
            self.new_start_pos,
        );
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        move_between(
            state,
            self.track_to,
            self.track_from,
            self.starting_range.with_start_pos(self.new_start_pos),
            self.starting_range.start,
        );
    }
}

// TODO see TrackAddOrRemove
#[derive(Clone)]
pub struct ClipAddOrRemove {
    track_id: Id<Track>,
    id: Id<Clip>,
    start_pos: i64,
    data: Option<Clip>,
    is_removal: bool,
}

impl ClipAddOrRemove {
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

    pub fn track_id(&self) -> Id<Track> {
        self.track_id
    }
    pub fn id(&self) -> Id<Clip> {
        self.id
    }
    pub fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to add clip to nonexistent track")
            .add_clip(
                self.id,
                self.start_pos,
                self.data
                    .take()
                    .expect("execute() called on empty ClipAddOrRemove"),
            );
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        self.data = Some(
            state
                .tracks
                .get_mut(self.track_id)
                .expect("tried to add clip to nonexistent track")
                .remove_clip(self.id, self.start_pos),
        );
    }
}

impl StateCommand for ClipAddOrRemove {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        if self.is_removal {
            self.execute_remove(state);
        } else {
            self.execute_add(state);
        }
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        if self.is_removal {
            self.execute_add(state);
        } else {
            self.execute_remove(state);
        }
    }
}
