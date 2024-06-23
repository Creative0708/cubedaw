use cubedaw_lib::{Cable, Id, Track};

#[derive(Clone)]
pub struct CableAddOrRemove {
    id: Id<Cable>,
    track_id: Id<Track>,
    data: Option<Cable>,
    is_removal: bool,
}

impl CableAddOrRemove {
    pub fn addition(id: Id<Cable>, data: Cable, track_id: Id<Track>) -> Self {
        Self {
            id,
            track_id,
            data: Some(data),
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

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        let cable_data = self
            .data
            .take()
            .expect("called execute_add on empty NodeAddOrRemove");

        state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to add node to nonexistent section")
            .patch
            .insert_cable(self.id, cable_data);
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        let cable_data = state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to remove node from nonexistent section")
            .patch
            .take_cable(self.id);

        if self.data.replace(cable_data).is_some() {
            panic!("called execute_remove on nonempty NodeAddOrRemove");
        }
    }
}

impl crate::StateCommand for CableAddOrRemove {
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
