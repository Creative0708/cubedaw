use cubedaw_lib::{Id, Range, Section, Track};

use crate::StateCommand;

#[derive(Clone)]
pub struct SectionMove {
    track_from: Id<Track>,
    track_to: Id<Track>,
    starting_range: Range,
    new_start_pos: i64,
}

impl SectionMove {
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
    let track_from = state.tracks.force_get_section_mut(track_from_id);
    if track_from_id == track_to_id {
        track_from.move_section(starting_range, new_start_pos);
    } else {
        let (section_id, section) = track_from.remove_section_from_range(starting_range);

        let track_to = state.tracks.force_get_section_mut(track_to_id);
        track_to.add_section(section_id, new_start_pos, section);
    }
}

impl StateCommand for SectionMove {
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
