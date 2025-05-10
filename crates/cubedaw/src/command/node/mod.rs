use cubedaw_lib::{Buffer, Id, Node, NodeData, Track};
use cubedaw_worker::command::{ActionType, StateCommand, StateCommandWrapper};
use egui::Vec2;

use crate::{state::ui::NodeUiState, util::Select};

use super::UiStateCommand;

#[derive(Clone)]
pub struct NodeStateUpdate {
    id: Id<Node>,
    track_id: Id<Track>,
    data: Box<Buffer>,
    input_values: Vec<f32>,
    old_input_values: Vec<f32>,
    num_outputs: u32,
    old_num_outputs: u32,
}

impl NodeStateUpdate {
    pub fn new(
        id: Id<Node>,
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
    fn node<'a>(&mut self, state: &'a mut cubedaw_lib::State) -> Option<&'a mut Node> {
        Some(
            state
                .tracks
                .get_mut(self.track_id)?
                .patch
                .node_entry_mut(self.id)?,
        )
    }
}

impl StateCommand for NodeStateUpdate {
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionType) {
        if let Some(node) = self.node(state) {
            let (input_values, num_outputs) = match action {
                ActionType::Execute => (&mut self.input_values, self.num_outputs),
                ActionType::Rollback => (&mut self.old_input_values, self.old_num_outputs),
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

#[derive(Clone)]
pub struct NoUiNodeAddOrRemove {
    id: Id<Node>,
    track_id: Id<Track>,
    data: Option<NodeData>,
    inputs: Vec<f32>,
    num_outputs: u32,
    is_removal: bool,
}

impl NoUiNodeAddOrRemove {
    pub fn addition(
        id: Id<Node>,
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
    pub fn removal(id: Id<Node>, track_id: Id<Track>) -> Self {
        Self {
            id,
            track_id,
            data: None,
            inputs: Vec::new(),
            num_outputs: 0,
            is_removal: true,
        }
    }

    pub fn id(&self) -> Id<Node> {
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
}

impl StateCommand for NoUiNodeAddOrRemove {
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionType) {
        if self.is_removal ^ action.is_rollback() {
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
        } else {
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
    }
}

#[derive(Clone)]
pub struct NodeBiasChange {
    id: Id<Node>,
    track_id: Id<Track>,
    input_index: u32,
    old_value: f32,
    new_value: f32,
}

impl NodeBiasChange {
    pub fn new(
        id: Id<Node>,
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
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionType) {
        let Some(input) = self.get_input(state) else {
            return;
        };
        input.bias = match action {
            ActionType::Execute => self.new_value,
            ActionType::Rollback => self.old_value,
        }
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
    id: Id<Node>,
    track_id: Id<Track>,
    input_index: u32,
    cable_index: u32,
    old_value: f32,
    new_value: f32,
}

impl NodeMultiplierChange {
    pub fn new(
        id: Id<Node>,
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
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionType) {
        let Some(multiplier) = self.get_multiplier(state) else {
            return;
        };
        *multiplier = match action {
            ActionType::Execute => self.new_value,
            ActionType::Rollback => self.old_value,
        }
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

pub struct UiNodeAddOrRemove {
    inner: NoUiNodeAddOrRemove,
    ui_data: Option<NodeUiState>,
}

impl UiNodeAddOrRemove {
    pub fn addition(
        id: Id<Node>,
        data: NodeData,
        inputs: Vec<f32>,
        num_outputs: u32,
        track_id: Id<Track>,
        ui_state: NodeUiState,
    ) -> Self {
        Self {
            inner: NoUiNodeAddOrRemove::addition(id, data, inputs, num_outputs, track_id),
            ui_data: Some(ui_state),
        }
    }
    pub fn removal(id: Id<Node>, track_id: Id<Track>) -> Self {
        Self {
            inner: NoUiNodeAddOrRemove::removal(id, track_id),
            ui_data: None,
        }
    }
}

impl UiStateCommand for UiNodeAddOrRemove {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        ephemeral_state: &mut crate::EphemeralState,
        action: ActionType,
    ) {
        let nodes = &mut ui_state
            .tracks
            .get_mut(self.inner.track_id())
            .expect("nonexistent track")
            .patch
            .nodes;
        if self.inner.is_removal() ^ action.is_rollback() {
            self.ui_data = nodes.remove(self.inner.id());

            if let Some(track) = ephemeral_state.tracks.get_mut(self.inner.track_id()) {
                track.patch.nodes.remove(self.inner.id());
            }
        } else {
            nodes.insert(
                self.inner.id(),
                self.ui_data
                    .take()
                    .expect("called execute_add() on empty UiNodeAddOrRemove"),
            );

            if let Some(track) = ephemeral_state.tracks.get_mut(self.inner.track_id()) {
                track
                    .patch
                    .nodes
                    .insert(self.inner.id(), Default::default());
            }
        }
    }
    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        Some(&mut self.inner)
    }
}

pub struct NodeSelect {
    track_id: Id<Track>,
    id: Id<Node>,
    select: Select,
}

impl NodeSelect {
    pub fn new(track_id: Id<Track>, id: Id<Node>, select: Select) -> Self {
        Self {
            track_id,
            id,
            select,
        }
    }
}

impl UiStateCommand for NodeSelect {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
        action: ActionType,
    ) {
        let Some(node_ui) = ui_state
            .tracks
            .force_get_mut(self.track_id)
            .patch
            .nodes
            .get_mut(self.id)
        else {
            return;
        };

        node_ui.select = self.select ^ action.is_rollback();
    }
}

pub struct UiNodeMove {
    id: Id<Node>,
    track_id: Id<Track>,
    offset: Vec2,
}

impl UiNodeMove {
    pub fn new(id: Id<Node>, track_id: Id<Track>, offset: Vec2) -> Self {
        Self {
            id,
            track_id,
            offset,
        }
    }

    fn node_ui<'a>(&self, ui_state: &'a mut crate::UiState) -> Option<&'a mut NodeUiState> {
        Some(
            ui_state
                .tracks
                .get_mut(self.track_id)?
                .patch
                .nodes
                .get_mut(self.id)?,
        )
    }
}

impl UiStateCommand for UiNodeMove {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
        action: ActionType,
    ) {
        if let Some(node) = self.node_ui(ui_state) {
            node.pos += match action {
                ActionType::Execute => self.offset,
                ActionType::Rollback => -self.offset,
            }
        }
    }
}
