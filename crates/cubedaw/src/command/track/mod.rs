use cubedaw_command::track::TrackAddOrRemove;
use cubedaw_lib::{Id, NodeData, Track};

use crate::{registry::NodeRegistry, state::ui::TrackUiState};

use super::UiStateCommand;
pub struct UiTrackAddOrRemove {
    inner: TrackAddOrRemove,
    ui_data: Option<TrackUiState>,
    // where the track is inserted in the track list
    insertion_pos: u32,
}

impl UiTrackAddOrRemove {
    pub fn addition(
        id: Id<Track>,
        data: Track,
        ui_data: TrackUiState,
        parent_track: Option<Id<Track>>,
        insertion_pos: u32,
    ) -> Self {
        Self {
            inner: TrackAddOrRemove::addition(id, data, parent_track),
            ui_data: Some(ui_data),
            insertion_pos,
        }
    }
    pub fn removal(id: Id<Track>, parent_track: Option<Id<Track>>) -> Self {
        Self {
            inner: TrackAddOrRemove::removal(id, parent_track),
            ui_data: None,
            insertion_pos: 0,
        }
    }

    pub fn add_generic_section_track(
        id: Id<Track>,
        parent_track: Option<Id<Track>>,
        insertion_pos: u32,
        _node_registry: &NodeRegistry,
    ) -> Self {
        let id_note_output = Id::arbitrary();
        let id_track_output = Id::arbitrary();
        Self::addition(
            id,
            cubedaw_lib::Track::new_section({
                let mut patch = cubedaw_lib::Patch::default();

                patch.insert_node(
                    id_note_output,
                    NodeData::new_disconnected(
                        resourcekey::literal!("builtin:note_output"),
                        Box::new([]),
                    ),
                    vec![0.0],
                    1,
                );
                patch.insert_node(
                    id_track_output,
                    NodeData::new_disconnected(
                        resourcekey::literal!("builtin:track_output"),
                        Box::new([]),
                    ),
                    vec![0.0],
                    0,
                );
                patch
                    .insert_cable(
                        Id::arbitrary(),
                        cubedaw_lib::Cable::one(id_note_output, id_track_output),
                    )
                    .multiplier = 1.0;

                patch
            }),
            crate::state::ui::TrackUiState {
                name: format!("Track {:04x}", id.raw().get() >> 48),
                patch: crate::state::ui::PatchUiState {
                    nodes: {
                        let mut map = cubedaw_lib::IdMap::new();
                        map.insert(
                            id_note_output,
                            crate::state::ui::NodeUiState {
                                selected: false,
                                pos: egui::pos2(-80.0, 0.0),
                                width: 128.0,
                            },
                        );
                        map.insert(
                            id_track_output,
                            crate::state::ui::NodeUiState {
                                selected: false,
                                pos: egui::pos2(240.0, 0.0),
                                width: 128.0,
                            },
                        );
                        map
                    },
                },
                ..Default::default()
            },
            parent_track,
            insertion_pos,
        )
    }

    pub fn add_generic_group_track(
        id: Id<Track>,
        parent_track: Option<Id<Track>>,
        insertion_pos: u32,
        _node_registry: &NodeRegistry,
    ) -> Self {
        let id_track_input = Id::arbitrary();
        let id_track_output = Id::arbitrary();
        Self::addition(
            id,
            cubedaw_lib::Track::new_group({
                let mut patch = cubedaw_lib::Patch::default();

                patch.insert_node(
                    id_track_input,
                    NodeData::new_disconnected(
                        resourcekey::literal!("builtin:track_input"),
                        Box::new([]),
                    ),
                    vec![0.0],
                    1,
                );
                patch.insert_node(
                    id_track_output,
                    NodeData::new_disconnected(
                        resourcekey::literal!("builtin:track_output"),
                        Box::new([]),
                    ),
                    vec![0.0],
                    0,
                );
                patch
                    .insert_cable(
                        Id::arbitrary(),
                        cubedaw_lib::Cable::one(id_track_input, id_track_output),
                    )
                    .multiplier = 1.0;

                patch
            }),
            crate::state::ui::TrackUiState {
                name: format!("Track {:04x}", id.raw().get() >> 48),
                patch: crate::state::ui::PatchUiState {
                    nodes: {
                        let mut map = cubedaw_lib::IdMap::new();
                        map.insert(
                            id_track_input,
                            crate::state::ui::NodeUiState {
                                selected: false,
                                pos: egui::pos2(-80.0, 0.0),
                                width: 128.0,
                            },
                        );
                        map.insert(
                            id_track_output,
                            crate::state::ui::NodeUiState {
                                selected: false,
                                pos: egui::pos2(240.0, 0.0),
                                width: 128.0,
                            },
                        );
                        map
                    },
                },
                ..Default::default()
            },
            parent_track,
            insertion_pos,
        )
    }

    fn execute_add(
        &mut self,
        ui_state: &mut crate::UiState,
        ephemeral_state: &mut crate::EphemeralState,
    ) {
        ui_state
            .tracks
            .insert(self.inner.id(), self.ui_data.take().unwrap_or_default());
        if let Some(parent_track) = self.inner.parent_track() {
            let parent_track_ui = ui_state.tracks.force_get_mut(parent_track);
            parent_track_ui.track_list.insert(
                (self.insertion_pos as usize).min(parent_track_ui.track_list.len()),
                self.inner.id(),
            );
        }

        ephemeral_state
            .tracks
            .insert(self.inner.id(), Default::default());
    }
    fn execute_remove(
        &mut self,
        ui_state: &mut crate::UiState,
        ephemeral_state: &mut crate::EphemeralState,
    ) {
        self.ui_data = ui_state.tracks.remove(self.inner.id());
        if let Some(parent_track) = self.inner.parent_track() {
            ui_state
                .tracks
                .force_get_mut(parent_track)
                .track_list
                .retain(|&id| id != self.inner.id());
        }

        ephemeral_state.tracks.remove(self.inner.id());
    }
}

impl UiStateCommand for UiTrackAddOrRemove {
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

pub struct UiTrackSelect {
    id: Id<Track>,
    selected: bool,
}

impl UiTrackSelect {
    pub fn new(id: Id<Track>, selected: bool) -> Self {
        Self { id, selected }
    }
}

impl UiStateCommand for UiTrackSelect {
    fn ui_execute(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = ui_state.tracks.get_mut(self.id) {
            ui_data.selected = self.selected;
        }
    }
    fn ui_rollback(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
    ) {
        if let Some(ui_data) = ui_state.tracks.get_mut(self.id) {
            ui_data.selected = !self.selected;
        }
    }
}
