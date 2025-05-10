use cubedaw_lib::{Cable, CableConnection, Id, Track};
use cubedaw_worker::command::StateCommand;

#[derive(Clone)]
pub struct CableAddOrRemove {
    id: Id<Cable>,
    track_id: Id<Track>,
    data: Option<(Cable, CableConnection)>,
    is_removal: bool,
}

impl CableAddOrRemove {
    pub fn addition(
        id: Id<Cable>,
        data: Cable,
        conn: CableConnection,
        track_id: Id<Track>,
    ) -> Self {
        Self {
            id,
            track_id,
            data: Some((data, conn)),
            is_removal: false,
        }
    }
    pub fn removal(id: Id<Cable>, track_id: Id<Track>) -> Self {
        Self {
            id,
            track_id,
            data: None,
            is_removal: true,
        }
    }

    pub fn id(&self) -> Id<Cable> {
        self.id
    }
    pub fn track_id(&self) -> Id<Track> {
        self.track_id
    }
    pub fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {}
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {}
}

impl StateCommand for CableAddOrRemove {
    fn run(&mut self, state: &mut cubedaw_lib::State, action: cubedaw_worker::command::ActionType) {
        let track = state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to add node to nonexistent clip");
        if self.is_removal ^ action.is_rollback() {
            let cable_data = track.patch.take_cable(self.id);

            if self.data.replace(cable_data).is_some() {
                panic!("called execute_remove on nonempty NodeAddOrRemove");
            }
        } else {
            let (cable, conn) = self
                .data
                .take()
                .expect("called execute_add on empty NodeAddOrRemove");
            track.patch.insert_cable(self.id, cable, conn);
        }
    }
}
