use cubedaw_lib::{Id, NodeData, NodeStateWrapper, Track};

use crate::StateCommand;

#[derive(Clone)]
pub struct NodeStateUpdate {
    id: Id<NodeData>,
    track_id: Id<Track>,
    data: Box<dyn NodeStateWrapper>,
    input_values: Vec<f32>,
    old_input_values: Vec<f32>,
    num_outputs: u32,
    old_num_outputs: u32,
}

impl NodeStateUpdate {
    pub fn new(
        id: Id<NodeData>,
        track_id: Id<Track>,
        data: Box<dyn NodeStateWrapper>,
        input_values: Vec<f32>,
        old_input_values: Vec<f32>,
        num_outputs: u32,
        old_num_outputs: u32,
    ) -> Self {
        Self {
            track_id,
            id,
            data,
            input_values,
            old_input_values,
            num_outputs,
            old_num_outputs,
        }
    }
}

impl NodeStateUpdate {
    pub fn track_id(&self) -> Id<Track> {
        self.track_id
    }
    pub fn id(&self) -> Id<NodeData> {
        self.id
    }

    fn node<'a>(
        &mut self,
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

    fn do_the_thing(&mut self, state: &mut cubedaw_lib::State, is_rollback: bool) {
        if let Some(node) = self.node(state) {
            let (input_values, num_outputs) = if is_rollback {
                (&mut self.old_input_values, self.old_num_outputs)
            } else {
                (&mut self.input_values, self.num_outputs)
            };

            node.inputs.resize(input_values.len(), Default::default());
            for (input, &value) in node.inputs.iter_mut().zip(input_values.iter()) {
                input.value = value;
            }

            node.outputs
                .resize(num_outputs as usize, Default::default());

            if NodeStateWrapper::type_id(self.data.as_ref())
                != NodeStateWrapper::type_id(node.inner.as_ref())
            {
                panic!("tried to replace NodeData with NodeData of different type")
            }
            core::mem::swap(&mut self.data, &mut node.inner);
        }
    }
}

impl StateCommand for NodeStateUpdate {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        self.do_the_thing(state, false);
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        self.do_the_thing(state, true);
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
