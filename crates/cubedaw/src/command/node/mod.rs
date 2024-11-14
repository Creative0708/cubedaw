use cubedaw_command::node::NodeAddOrRemove;
use cubedaw_lib::{Id, IdMap, NodeData, NodeEntry, Track};
use egui::Vec2;

use crate::state::ui::NodeUiState;

use super::UiStateCommand;

pub struct UiNodeAddOrRemove {
    inner: NodeAddOrRemove,
    ui_data: Option<NodeUiState>,
    num_inputs: u32,
    num_outputs: u32,
}

impl UiNodeAddOrRemove {
    pub fn addition(
        id: Id<NodeEntry>,
        data: NodeData,
        inputs: Vec<f32>,
        num_outputs: u32,
        track_id: Id<Track>,
        ui_state: NodeUiState,
    ) -> Self {
        Self {
            num_inputs: inputs.len() as u32,
            num_outputs,

            inner: NodeAddOrRemove::addition(id, data, inputs, num_outputs, track_id),
            ui_data: Some(ui_state),
        }
    }
    pub fn removal(id: Id<NodeEntry>, track_id: Id<Track>) -> Self {
        Self {
            inner: NodeAddOrRemove::removal(id, track_id),
            ui_data: None,

            num_inputs: 0,
            num_outputs: 0,
        }
    }

    fn nodes<'a>(&self, ui_state: &'a mut crate::UiState) -> &'a mut IdMap<NodeEntry, NodeUiState> {
        &mut ui_state
            .tracks
            .get_mut(self.inner.track_id())
            .expect("nonexistent track")
            .patch
            .nodes
    }

    fn execute_add(
        &mut self,
        ui_state: &mut crate::UiState,
        ephemeral_state: &mut crate::EphemeralState,
    ) {
        self.nodes(ui_state).insert(
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
    fn execute_remove(
        &mut self,
        ui_state: &mut crate::UiState,
        ephemeral_state: &mut crate::EphemeralState,
    ) {
        self.ui_data = self.nodes(ui_state).remove(self.inner.id());

        if let Some(track) = ephemeral_state.tracks.get_mut(self.inner.track_id()) {
            track.patch.nodes.remove(self.inner.id());
        }
    }
}

impl UiStateCommand for UiNodeAddOrRemove {
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        ephemeral_state: &mut crate::EphemeralState,
    ) {
        if self.inner.is_removal() {
            self.execute_remove(ui_state, ephemeral_state);
        } else {
            self.execute_add(ui_state, ephemeral_state);
        }
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        ephemeral_state: &mut crate::EphemeralState,
    ) {
        if self.inner.is_removal() {
            self.execute_add(ui_state, ephemeral_state);
        } else {
            self.execute_remove(ui_state, ephemeral_state);
        }
    }
    fn inner(&mut self) -> Option<&mut dyn cubedaw_command::StateCommandWrapper> {
        Some(&mut self.inner)
    }
}

pub struct UiNodeSelect {
    track_id: Id<Track>,
    id: Id<NodeEntry>,
    selected: bool,
}

impl UiNodeSelect {
    pub fn new(track_id: Id<Track>, id: Id<NodeEntry>, selected: bool) -> Self {
        Self {
            track_id,
            id,
            selected,
        }
    }

    fn node<'a>(&self, ui_state: &'a mut crate::UiState) -> Option<&'a mut NodeUiState> {
        ui_state
            .tracks
            .force_get_mut(self.track_id)
            .patch
            .nodes
            .get_mut(self.id)
    }
}

impl UiStateCommand for UiNodeSelect {
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = self.node(ui_state) {
            ui_data.selected = self.selected;
        }
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = self.node(ui_state) {
            ui_data.selected = !self.selected;
        }
    }
}

pub struct UiNodeMove {
    id: Id<NodeEntry>,
    track_id: Id<Track>,
    offset: Vec2,
}

impl UiNodeMove {
    pub fn new(id: Id<NodeEntry>, track_id: Id<Track>, offset: Vec2) -> Self {
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
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(node) = self.node_ui(ui_state) {
            node.pos += self.offset;
        }
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(node) = self.node_ui(ui_state) {
            node.pos -= self.offset;
        }
    }
}
