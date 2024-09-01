use cubedaw_lib::{Id, Track};

use crate::StateCommand;

#[derive(Clone)]
pub struct TrackAddOrRemove {
    id: Id<Track>,
    data: Option<Track>,
    parent_track: Option<Id<Track>>,
    is_removal: bool,
}

impl TrackAddOrRemove {
    pub fn addition(id: Id<Track>, data: Track, parent_track: Option<Id<Track>>) -> Self {
        Self {
            id,
            data: Some(data),
            parent_track,
            is_removal: false,
        }
    }
    pub fn removal(id: Id<Track>, parent_track: Option<Id<Track>>) -> Self {
        Self {
            id,
            data: None,
            parent_track,
            is_removal: true,
        }
    }
    pub const fn id(&self) -> Id<Track> {
        self.id
    }
    pub const fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn parent_track<'a>(
        &self,
        state: &'a mut cubedaw_lib::State,
    ) -> &'a mut cubedaw_lib::GroupTrack {
        match self.parent_track {
            Some(parent_track) => state
                .tracks
                .force_get_mut(parent_track)
                .inner
                .group_mut()
                .unwrap(),
            None => &mut state.root_track,
        }
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        assert!(
            self.parent_track(state).children.insert(self.id),
            "tried to add track as child twice"
        );
        state.tracks.insert(
            self.id,
            self.data
                .take()
                .expect("execute() called on empty TrackAddOrRemove"),
        );
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        assert!(
            !self.parent_track(state).children.remove(&self.id),
            "tried to remove nonexistent child"
        );
        self.data = Some(
            state
                .tracks
                .remove(self.id)
                .expect("tried to delete nonexistent track"),
        );
    }
}

impl StateCommand for TrackAddOrRemove {
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
