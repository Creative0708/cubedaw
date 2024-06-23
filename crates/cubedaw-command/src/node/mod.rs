use cubedaw_lib::{Id, NodeData, NodeInput, NodeStateWrapper, Patch, Track};

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
    pub fn track_id(&self) -> Id<Track> {
        self.track_id
    }
    pub fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn get_patch<'a>(&self, state: &'a mut cubedaw_lib::State) -> &'a mut cubedaw_lib::Patch {
        &mut state
            .tracks
            .get_mut(self.track_id)
            .expect("tried to add node to nonexistent patch")
            .patch
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        let node_data = self
            .data
            .take()
            .expect("called execute_add on empty NodeAddOrRemove");

        self.get_patch(state).insert_node(self.id, node_data);
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        let node_data = self.get_patch(state).take_node(self.id);

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

#[derive(Clone)]
pub struct NodeInputChange {
    id: Id<NodeData>,
    track_id: Id<Track>,
    input_index: usize,
    old_value: f32,
    new_value: f32,
}

impl NodeInputChange {
    pub fn new(
        id: Id<NodeData>,
        track_id: Id<Track>,
        input_index: usize,
        old_value: f32,
        new_value: f32,
    ) -> Self {
        Self {
            id,
            track_id,
            input_index,
            old_value,
            new_value,
        }
    }

    fn get_input<'a>(
        &self,
        state: &'a mut cubedaw_lib::State,
    ) -> Option<&'a mut cubedaw_lib::NodeInput> {
        Some(
            state
                .tracks
                .get_mut(self.track_id)?
                .patch
                .node_mut(self.id)?
                .inputs
                .get_mut(self.input_index)?,
        )
    }
}

impl StateCommand for NodeInputChange {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        let Some(input) = self.get_input(state) else {
            return;
        };
        input.value = self.new_value;
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        let Some(input) = self.get_input(state) else {
            return;
        };
        input.value = self.old_value;
    }

    fn try_merge(&mut self, other: &Self) -> bool {
        if (self.id, self.track_id) == (other.id, other.track_id) {
            self.new_value = other.new_value;
            true
        } else {
            false
        }
    }
}

#[derive(Clone)]
pub struct NodeInputAddOrRemove {
    id: Id<NodeData>,
    track_id: Id<Track>,
    input_index: usize,
    value: f32,

    is_removal: bool,
}

impl NodeInputAddOrRemove {
    pub fn addition(id: Id<NodeData>, track_id: Id<Track>, input_index: usize, value: f32) -> Self {
        Self {
            id,
            track_id,
            input_index,
            value,
            is_removal: false,
        }
    }
    pub fn removal(
        id: Id<NodeData>,
        track_id: Id<Track>,
        input_index: usize,
        old_value: f32,
    ) -> Self {
        Self {
            id,
            track_id,
            input_index,
            value: old_value,
            is_removal: true,
        }
    }

    fn get_node<'a>(
        &self,
        state: &'a mut cubedaw_lib::State,
    ) -> Option<&'a mut cubedaw_lib::NodeData> {
        Some(
            state
                .tracks
                .get_mut(self.track_id)?
                .patch
                .node_mut(self.id)?,
        )
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        let Some(node) = self.get_node(state) else {
            return;
        };

        assert!(
            node.inputs.len() == self.input_index,
            "tried to add input in the middle of the input list"
        );

        node.inputs.push(NodeInput {
            value: self.value,
            ..Default::default()
        });
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        let Some(node) = self.get_node(state) else {
            return;
        };

        assert!(
            node.inputs.len() == self.input_index + 1,
            "tried to delete input from the middle of the input list"
        );
        let popped = node.inputs.pop();
        assert!(
            popped.is_some_and(|popped| popped.connections.is_empty()),
            "tried to delete input still connected to cable"
        );
    }
}

impl StateCommand for NodeInputAddOrRemove {
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
