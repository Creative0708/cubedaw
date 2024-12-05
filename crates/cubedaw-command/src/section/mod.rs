use cubedaw_lib::{Id, Range, Section, Track};

use crate::StateCommand;

#[derive(Clone)]
pub struct SectionMove {
    track_id: Id<Track>,
    starting_range: Range,
    new_start_pos: i64,
}

impl SectionMove {
    pub fn new(track_id: Id<Track>, starting_range: Range, new_start_pos: i64) -> Self {
        Self {
            track_id,
            starting_range,
            new_start_pos,
        }
    }
}

impl StateCommand for SectionMove {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        let track = state
            .tracks
            .get_mut(self.track_id)
            .expect("nonexistent track id in SectionMove")
            .inner
            .section_mut()
            .expect("track doesn't have sections");

        track.move_section(self.starting_range, self.new_start_pos);
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        let track = state
            .tracks
            .get_mut(self.track_id)
            .expect("nonexistent track id in SectionMove")
            .inner
            .section_mut()
            .expect("track isn't a synth track");

        track.move_section(
            self.starting_range + (self.new_start_pos - self.starting_range.start),
            self.starting_range.start,
        );
    }
}

// TODO see TrackAddOrRemove
#[derive(Clone)]
pub struct SectionAddOrRemove {
    track_id: Id<Track>,
    id: Id<Section>,
    start_pos: i64,
    data: Option<Section>,
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

    pub fn track_id(&self) -> Id<Track> {
        self.track_id
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
            .inner
            .section_mut()
            .expect("track isn't a section track")
            .add_section(
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
                .inner
                .section_mut()
                .expect("track isn't a section track")
                .remove_section(self.id, self.start_pos),
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
