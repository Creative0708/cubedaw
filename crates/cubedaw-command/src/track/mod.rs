use cubedaw_lib::{Id, Track};

use crate::StateCommand;

// TODO revise this name; maybe it's fine but it feels awkward
#[derive(Clone)]
pub struct TrackAddOrRemove {
    id: Id<Track>,
    data: Option<Track>,
    is_removal: bool,
}

impl TrackAddOrRemove {
    pub fn addition(id: Id<Track>, data: Track) -> Self {
        Self {
            id,
            data: Some(data),
            is_removal: false,
        }
    }
    pub fn removal(id: Id<Track>) -> Self {
        Self {
            id,
            data: None,
            is_removal: true,
        }
    }
    pub const fn id(&self) -> Id<Track> {
        self.id
    }
    pub const fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        state.tracks.insert(
            self.id,
            self.data
                .take()
                .expect("execute() called on empty TrackAddOrRemove"),
        );
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
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
