use cubedaw_command::node::NodeAddOrRemove;
use cubedaw_lib::{Id, IdMap, NodeData, Track};

use crate::state::ui::NodeUiState;

use super::UiStateCommand;

pub struct UiNodeAddOrRemove {
    inner: NodeAddOrRemove,
    ui_data: Option<NodeUiState>,
}

impl UiNodeAddOrRemove {
    pub fn addition(
        id: Id<NodeData>,
        data: NodeData,
        track_id: Id<Track>,
        ui_state: NodeUiState,
    ) -> Self {
        Self {
            inner: NodeAddOrRemove::addition(id, data, track_id),
            ui_data: Some(ui_state),
        }
    }
    pub fn removal(id: Id<NodeData>, track_id: Id<Track>) -> Self {
        Self {
            inner: NodeAddOrRemove::removal(id, track_id),
            ui_data: None,
        }
    }

    fn nodes<'a>(&self, ui_state: &'a mut crate::UiState) -> &'a mut IdMap<NodeData, NodeUiState> {
        &mut ui_state
            .tracks
            .get_mut(self.inner.track_id())
            .expect("nonexistent track")
            .patch
            .nodes
    }

    fn execute_add(&mut self, ui_state: &mut crate::UiState) {
        self.nodes(ui_state).insert(
            self.inner.id(),
            self.ui_data
                .take()
                .expect("called execute_add() on empty UiNodeAddOrRemove"),
        );
    }
    fn execute_remove(&mut self, ui_state: &mut crate::UiState) {
        self.ui_data = self.nodes(ui_state).remove(self.inner.id());
    }
}

impl UiStateCommand for UiNodeAddOrRemove {
    fn ui_execute(&mut self, ui_state: &mut crate::UiState) {
        if self.inner.is_removal() {
            self.execute_remove(ui_state);
        } else {
            self.execute_add(ui_state);
        }
    }
    fn ui_rollback(&mut self, ui_state: &mut crate::UiState) {
        if self.inner.is_removal() {
            self.execute_add(ui_state);
        } else {
            self.execute_remove(ui_state);
        }
    }
    fn inner(&mut self) -> Option<&mut dyn cubedaw_command::StateCommandWrapper> {
        Some(&mut self.inner)
    }
}

pub struct UiNodeSelect {
    track_id: Id<Track>,
    id: Id<NodeData>,
    selected: bool,
}

impl UiNodeSelect {
    pub fn new(track_id: Id<Track>, id: Id<NodeData>, selected: bool) -> Self {
        Self {
            track_id,
            id,
            selected,
        }
    }

    fn node<'a>(&self, ui_state: &'a mut crate::UiState) -> Option<&'a mut NodeUiState> {
        ui_state
            .tracks
            .get_mut(self.track_id)
            .expect("tried selecting node on nonexistent track")
            .patch
            .nodes
            .get_mut(self.id)
    }
}

impl UiStateCommand for UiNodeSelect {
    fn ui_execute(&mut self, ui_state: &mut crate::UiState) {
        if let Some(ui_data) = self.node(ui_state) {
            ui_data.selected = self.selected;
        }
    }
    fn ui_rollback(&mut self, ui_state: &mut crate::UiState) {
        if let Some(ui_data) = self.node(ui_state) {
            ui_data.selected = !self.selected;
        }
    }
}
