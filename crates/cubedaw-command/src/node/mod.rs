use cubedaw_lib::{Buffer, Id, NodeData, NodeEntry, Track};

use crate::StateCommand;

#[derive(Clone)]
pub struct NodeStateUpdate {
    id: Id<NodeEntry>,
    track_id: Id<Track>,
    data: Box<Buffer>,
    input_values: Vec<f32>,
    old_input_values: Vec<f32>,
    num_outputs: u32,
    old_num_outputs: u32,
}

impl NodeStateUpdate {
    pub fn new(
        id: Id<NodeEntry>,
        track_id: Id<Track>,
        data: Box<Buffer>,
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
    pub fn id(&self) -> Id<NodeEntry> {
        self.id
    }

    fn node<'a>(&mut self, state: &'a mut cubedaw_lib::State) -> Option<&'a mut NodeEntry> {
        Some(
            state
                .tracks
                .get_mut(self.track_id)?
                .patch
                .node_entry_mut(self.id)?,
        )
    }

    fn do_the_thing(&mut self, state: &mut cubedaw_lib::State, is_rollback: bool) {
        if let Some(node) = self.node(state) {
            let (input_values, num_outputs) = if is_rollback {
                (&mut self.old_input_values, self.old_num_outputs)
            } else {
                (&mut self.input_values, self.num_outputs)
            };

            {
                while node.inputs().len() > input_values.len() {
                    assert!(
                        node.pop_input().is_some(),
                        "NodeStateUpdate tried to remove connected input"
                    );
                }
                for (input, &value) in node.inputs_mut().iter_mut().zip(input_values.iter()) {
                    input.bias = value;
                }
                while node.inputs().len() < input_values.len() {
                    node.push_input(input_values[node.inputs().len()]);
                }
            }

            {
                while node.outputs().len() > num_outputs as usize {
                    assert!(
                        node.pop_output().is_some(),
                        "NodeStateUpdate tried to remove connected output"
                    );
                }
                while node.outputs().len() < num_outputs as usize {
                    node.push_output();
                }
            }

            core::mem::swap(&mut self.data, &mut node.data.inner);
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
    id: Id<NodeEntry>,
    track_id: Id<Track>,
    data: Option<NodeData>,
    inputs: Vec<f32>,
    num_outputs: u32,
    is_removal: bool,
}

impl NodeAddOrRemove {
    pub fn addition(
        id: Id<NodeEntry>,
        data: NodeData,
        inputs: Vec<f32>,
        num_outputs: u32,
        track_id: Id<Track>,
    ) -> Self {
        Self {
            id,
            track_id,
            data: Some(data),
            inputs,
            num_outputs,
            is_removal: false,
        }
    }
    pub fn removal(id: Id<NodeEntry>, track_id: Id<Track>) -> Self {
        Self {
            id,
            track_id,
            data: None,
            inputs: Vec::new(),
            num_outputs: 0,
            is_removal: true,
        }
    }

    pub fn id(&self) -> Id<NodeEntry> {
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

        self.get_patch(state).insert_node(
            self.id,
            node_data,
            core::mem::replace(&mut self.inputs, Vec::new()),
            self.num_outputs,
        );
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        let node_data = self
            .get_patch(state)
            .remove_entry(self.id)
            .expect("tried to remove nonexistent node");

        assert!(self.inputs.is_empty());
        self.inputs
            .extend(node_data.inputs().iter().map(|input| input.bias));
        self.num_outputs = node_data.outputs().len() as u32;

        if self.data.replace(node_data.data).is_some() {
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
pub struct NodeBiasChange {
    id: Id<NodeEntry>,
    track_id: Id<Track>,
    input_index: u32,
    old_value: f32,
    new_value: f32,
}

impl NodeBiasChange {
    pub fn new(
        id: Id<NodeEntry>,
        track_id: Id<Track>,
        input_index: u32,
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
                .node_entry_mut(self.id)?
                .inputs_mut()
                .get_mut(self.input_index as usize)?,
        )
    }
}

impl StateCommand for NodeBiasChange {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        let Some(input) = self.get_input(state) else {
            return;
        };
        input.bias = self.new_value;
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        let Some(input) = self.get_input(state) else {
            return;
        };
        input.bias = self.old_value;
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
pub struct NodeMultiplierChange {
    id: Id<NodeEntry>,
    track_id: Id<Track>,
    input_index: u32,
    cable_index: u32,
    old_value: f32,
    new_value: f32,
}

impl NodeMultiplierChange {
    pub fn new(
        id: Id<NodeEntry>,
        track_id: Id<Track>,
        input_index: u32,
        cable_index: u32,
        old_value: f32,
        new_value: f32,
    ) -> Self {
        Self {
            id,
            track_id,
            input_index,
            cable_index,
            old_value,
            new_value,
        }
    }

    fn get_multiplier<'a>(&self, state: &'a mut cubedaw_lib::State) -> Option<&'a mut f32> {
        Some(
            &mut state
                .tracks
                .get_mut(self.track_id)?
                .patch
                .node_entry_mut(self.id)?
                .inputs_mut()
                .get_mut(self.input_index as usize)?
                .connections
                .get_mut(self.cable_index as usize)?
                .1
                .multiplier,
        )
    }
}

impl StateCommand for NodeMultiplierChange {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        let Some(multiplier) = self.get_multiplier(state) else {
            return;
        };
        *multiplier = self.new_value;
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        let Some(multiplier) = self.get_multiplier(state) else {
            return;
        };
        *multiplier = self.old_value;
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
