use cubedaw_lib::{Id, NodeData, Track};
use cubedaw_worker::command::{ActionDirection, StateCommand, StateCommandWrapper};

use crate::{registry::NodeRegistry, state::ui::TrackUiState, util::Select};

use super::UiStateCommand;

#[derive(Clone)]
struct NoUiTrackAddOrRemove {
    id: Id<Track>,
    data: Option<Track>,
    parent_track: Option<Id<Track>>,
    is_removal: bool,
}

impl NoUiTrackAddOrRemove {
    pub fn addition(id: Id<Track>, data: Track, parent_track: Option<Id<Track>>) -> Self {
        Self {
            id,
            data: Some(data),
            parent_track,
            is_removal: false,
        }
    }
    pub fn removal(id: Id<Track>, parent_track: Option<Id<Track>>) -> Self {
        Self {
            id,
            data: None,
            parent_track,
            is_removal: true,
        }
    }

    fn get_parent_track<'a>(
        &self,
        state: &'a mut cubedaw_lib::State,
    ) -> Option<&'a mut cubedaw_lib::Track> {
        Some(state.tracks.force_get_mut(self.parent_track?))
    }
}

impl StateCommand for NoUiTrackAddOrRemove {
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionDirection) {
        if self.is_removal ^ action.is_rollback() {
            if let Some(track) = self.get_parent_track(state) {
                let did_remove = track.children.remove(&self.id);
                assert!(did_remove, "tried to remove nonexistent child");
            }
            self.data = Some(
                state
                    .tracks
                    .remove(self.id)
                    .expect("tried to delete nonexistent track"),
            );
        } else {
            state.tracks.insert(
                self.id,
                self.data
                    .take()
                    .expect("execute() called on empty TrackAddOrRemove"),
            );
            match self.get_parent_track(state) {
                Some(track) => {
                    let did_insert = track.children.insert(self.id);
                    assert!(did_insert, "tried to add track as child twice");
                }
                None => {
                    assert!(
                        !state.tracks.has(state.root_track),
                        "tried to override root track"
                    );
                    state.root_track = self.id;
                }
            }
        }
    }
}

pub struct TrackAddOrRemove {
    inner: NoUiTrackAddOrRemove,
    ui_data: Option<TrackUiState>,
    // where the track is inserted in the track list
    insertion_pos: u32,
}

impl TrackAddOrRemove {
    pub fn addition(
        id: Id<Track>,
        data: Track,
        ui_data: TrackUiState,
        parent_track: Option<Id<Track>>,
        insertion_pos: u32,
    ) -> Self {
        Self {
            inner: NoUiTrackAddOrRemove::addition(id, data, parent_track),
            ui_data: Some(ui_data),
            insertion_pos,
        }
    }
    pub fn removal(id: Id<Track>, parent_track: Option<Id<Track>>) -> Self {
        Self {
            inner: NoUiTrackAddOrRemove::removal(id, parent_track),
            ui_data: None,
            insertion_pos: 0,
        }
    }

    pub fn add_generic_track(
        id: Id<Track>,
        parent_track: Option<Id<Track>>,
        insertion_pos: u32,
        _node_registry: &NodeRegistry,
    ) -> Self {
        let id_downmix = Id::arbitrary();
        let id_output = Id::arbitrary();
        Self::addition(
            id,
            cubedaw_lib::Track::new({
                let mut patch = cubedaw_lib::Patch::default();

                patch.insert_node(
                    id_downmix,
                    NodeData::new_disconnected(
                        resourcekey::literal!("builtin:downmix"),
                        Default::default(),
                    ),
                    vec![0.0],
                    1,
                );
                patch.insert_node(
                    id_output,
                    NodeData::new_disconnected(
                        resourcekey::literal!("builtin:output"),
                        Default::default(),
                    ),
                    vec![0.0],
                    0,
                );
                patch.insert_cable(
                    Id::arbitrary(),
                    cubedaw_lib::Cable::one(id_downmix, id_output),
                    cubedaw_lib::CableConnection { multiplier: 1.0 },
                );

                patch
            }),
            crate::state::ui::TrackUiState {
                name: format!("Track {:04x}", id.raw().get() >> 48),
                patch: crate::state::ui::PatchUiState {
                    nodes: {
                        let mut map = cubedaw_lib::IdMap::new();
                        map.insert(
                            id_downmix,
                            crate::state::ui::NodeUiState {
                                select: Select::Deselect,
                                pos: egui::pos2(-80.0, 0.0),
                                width: 128.0,
                            },
                        );
                        map.insert(
                            id_output,
                            crate::state::ui::NodeUiState {
                                select: Select::Deselect,
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
}

impl UiStateCommand for TrackAddOrRemove {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        ephemeral_state: &mut crate::EphemeralState,
        action: ActionDirection,
    ) {
        if self.inner.is_removal ^ action.is_rollback() {
            self.ui_data = ui_state.tracks.remove(self.inner.id);
            if let Some(parent_track) = self.inner.parent_track {
                ui_state
                    .tracks
                    .force_get_mut(parent_track)
                    .track_list
                    .retain(|&id| id != self.inner.id);
            }

            ephemeral_state.tracks.remove(self.inner.id);
        } else {
            ui_state
                .tracks
                .insert(self.inner.id, self.ui_data.take().unwrap_or_default());
            if let Some(parent_track) = self.inner.parent_track {
                let parent_track_ui = ui_state.tracks.force_get_mut(parent_track);
                parent_track_ui.track_list.insert(
                    (self.insertion_pos as usize).min(parent_track_ui.track_list.len()),
                    self.inner.id,
                );
            }

            ephemeral_state
                .tracks
                .insert(self.inner.id, Default::default());
        }
    }

    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        Some(&mut self.inner)
    }
}

pub struct TrackSelect {
    id: Id<Track>,
    select: Select,
}

impl TrackSelect {
    pub fn new(id: Id<Track>, select: Select) -> Self {
        Self { id, select }
    }
}

impl UiStateCommand for TrackSelect {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
        action: ActionDirection,
    ) {
        if let Some(ui_data) = ui_state.tracks.get_mut(self.id) {
            ui_data.select = self.select ^ action.is_rollback();
        }
    }
}
