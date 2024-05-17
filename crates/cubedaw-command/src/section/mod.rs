use cubedaw_lib::{Id, Range, Section, Track};

use crate::StateCommand;

pub struct SectionMove {
    track_id: Id<Track>,
    starting_range: Range,
    new_start_pos: i64,
}

impl SectionMove {
    pub fn new(track_id: Id<Track>, starting_range: Range, new_range: i64) -> Self {
        Self {
            track_id,
            starting_range,
            new_start_pos: new_range,
        }
    }
}

impl StateCommand for SectionMove {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        let track = state
            .tracks
            .get_mut(self.track_id)
            .expect("nonexistent track id in SectionMove");

        track.move_section(self.starting_range, self.new_start_pos);
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        let track = state
            .tracks
            .get_mut(self.track_id)
            .expect("nonexistent track id in SectionMove");

        track.move_section(
            self.starting_range + (self.new_start_pos - self.starting_range.start),
            self.starting_range.start,
        );
    }
}

// TODO see TrackAddOrRemove
pub struct SectionAddOrRemove {
    id: Id<Section>,
    start_pos: i64,
    data: Option<Section>,
    track_id: Id<Track>,
    is_removal: bool,
}

impl SectionAddOrRemove {
    pub fn addition(id: Id<Section>, start_pos: i64, data: Section, track_id: Id<Track>) -> Self {
        Self {
            id,
            start_pos,
            track_id,
            data: Some(data),
            is_removal: false,
        }
    }
    pub fn removal(id: Id<Section>, start_pos: i64, track_id: Id<Track>) -> Self {
        Self {
            start_pos,
            id,
            track_id,
            data: None,
            is_removal: true,
        }
    }

    pub fn id(&self) -> Id<Section> {
        self.id
    }
    pub fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to add section to nonexistent track")
            .add_section(
                &mut state.sections,
                self.id,
                self.start_pos,
                self.data
                    .take()
                    .expect("execute() called on empty SectionAddOrRemove"),
            );
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        self.data = Some(
            state
                .tracks
                .get_mut(self.track_id)
                .expect("tried to add section to nonexistent track")
                .remove_section(&mut state.sections, self.id, self.start_pos),
        );
    }
}

impl StateCommand for SectionAddOrRemove {
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
