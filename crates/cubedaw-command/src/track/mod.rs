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
    pub const fn parent_track(&self) -> Option<Id<Track>> {
        self.parent_track
    }

    fn get_parent_track<'a>(
        &self,
        state: &'a mut cubedaw_lib::State,
    ) -> Option<&'a mut cubedaw_lib::Track> {
        Some(state.tracks.force_get_mut(self.parent_track?))
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        state.tracks.insert(
            self.id,
            self.data
                .take()
                .expect("execute() called on empty TrackAddOrRemove"),
        );
        match self.get_parent_track(state) {
            Some(track) => {
                let did_insert = track.children.insert(self.id);
                assert!(did_insert, "tried to add track as child twice");
            }
            None => {
                assert!(
                    !state.tracks.has(state.root_track),
                    "tried to override root track"
                );
                state.root_track = self.id;
            }
        }
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        if let Some(track) = self.get_parent_track(state) {
            let did_remove = track.children.remove(&self.id);
            assert!(did_remove, "tried to remove nonexistent child");
        }
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
