use cubedaw_lib::{Id, NodeData, NodeStateWrapper, Track};

use crate::StateCommand;

#[derive(Clone)]
pub struct NodeUiUpdate {
    track_id: Id<Track>,
    id: Id<NodeData>,
    data: Box<dyn NodeStateWrapper>,
}

impl NodeUiUpdate {
    pub fn new(track_id: Id<Track>, id: Id<NodeData>, data: Box<dyn NodeStateWrapper>) -> Self {
        Self { track_id, id, data }
    }
}

impl NodeUiUpdate {
    fn swap_data(&mut self, state: &mut cubedaw_lib::State) {
        let node = state
            .tracks
            .get_mut(self.track_id)
            .expect("nonexistent track")
            .patch
            .node_mut(self.id)
            .expect("nonexistent node");
        if NodeStateWrapper::type_id(self.data.as_ref())
            != NodeStateWrapper::type_id(node.inner.as_ref())
        {
            panic!("tried to replace NodeData with NodeData of different type")
        }
        core::mem::swap(&mut self.data, &mut node.inner);
    }
}

impl StateCommand for NodeUiUpdate {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        self.swap_data(state);
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        self.swap_data(state);
    }
}

#[derive(Clone)]
pub struct NodeAddOrRemove {
    id: Id<NodeData>,
    track_id: Id<Track>,
    data: Option<NodeData>,
    is_removal: bool,
}

impl NodeAddOrRemove {
    pub fn addition(id: Id<NodeData>, data: NodeData, track_id: Id<Track>) -> Self {
        Self {
            id,
            track_id,
            data: Some(data),
            is_removal: false,
        }
    }
    pub fn removal(id: Id<NodeData>, track_id: Id<Track>) -> Self {
        Self {
            id,
            track_id,
            data: None,
            is_removal: true,
        }
    }

    pub fn id(&self) -> Id<NodeData> {
        self.id
    }
    pub fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        let node_data = self
            .data
            .take()
            .expect("called execute_add on empty NodeAddOrRemove");

        state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to add node to nonexistent section")
            .patch
            .insert_node(self.id, node_data);
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        let node_data = state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to remove node from nonexistent section")
            .patch
            .take_node(self.id);

        if self.data.replace(node_data).is_some() {
            panic!("called execute_remove on nonempty NodeAddOrRemove");
        }
    }
}

impl StateCommand for NodeAddOrRemove {
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
