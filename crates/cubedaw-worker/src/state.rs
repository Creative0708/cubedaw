use cubedaw_lib::{
    GroupTrack, Id, IdMap, NodeEntry, Note, Patch, Section, SectionTrack, State, Track, TrackInner,
};

use crate::{node_graph::ProcessedNodeGraph, WorkerOptions};

#[derive(Debug)]
pub struct WorkerState {
    pub section_tracks: IdMap<Track, WorkerSectionTrackState>,
    pub group_tracks: IdMap<Track, WorkerGroupTrackState>,
}

impl WorkerState {
    pub fn new(state: &State, options: &WorkerOptions) -> Self {
        let mut section_tracks = IdMap::new();
        let mut group_tracks = IdMap::new();
        for (&track_id, track) in &state.tracks {
            let mut node_map = IdMap::new();
            for (node_id, node) in track.patch.nodes() {
                node_map.insert(node_id, options.registry.create_node(node.data.key_id));
            }
            match track.inner {
                cubedaw_lib::TrackInner::Section(ref inner) => {
                    section_tracks.insert(
                        track_id,
                        WorkerSectionTrackState::from_patch(track, inner, options).unwrap_or_else(
                            || WorkerSectionTrackState::empty(track, inner, options),
                        ),
                    );
                }
                cubedaw_lib::TrackInner::Group(ref inner) => {
                    group_tracks.insert(
                        track_id,
                        WorkerGroupTrackState::from_patch(track, inner, options)
                            .unwrap_or_else(|| WorkerGroupTrackState::empty(track, inner, options)),
                    );
                }
            }
        }
        if let Some(a) = group_tracks.iter().next() {
            a.1.nodes.get_node(a.1.nodes.input_node().unwrap()).unwrap();
        }
        Self {
            section_tracks,
            group_tracks,
        }
    }

    pub fn sync_with(&mut self, state: &State, worker_options: &WorkerOptions) {
        let mut tracks_to_delete = Vec::new();
        for (&track_id, _) in &self.group_tracks {
            if !state.tracks.has(track_id) {
                tracks_to_delete.push(track_id);
            }
        }
        for track_id in tracks_to_delete.drain(..) {
            self.group_tracks.remove(track_id);
        }

        let mut notes_to_delete = Vec::new();
        for (&track_id, worker_track_data) in &mut self.section_tracks {
            match state.tracks.get(track_id) {
                Some(track) => {
                    let track_data = track.inner.section().unwrap();
                    for (&note_id, crate::WorkerNoteState { section_id, .. }) in
                        &worker_track_data.notes
                    {
                        if track_data
                            .section(*section_id)
                            .and_then(|section| section.note(note_id))
                            .is_none()
                        {
                            notes_to_delete.push(note_id);
                        }
                    }
                    for note_id in notes_to_delete.drain(..) {
                        worker_track_data.notes.remove(note_id);
                    }
                }
                None => {
                    tracks_to_delete.push(track_id);
                }
            }
        }
        for track_id in tracks_to_delete {
            self.section_tracks.remove(track_id);
        }

        for (&track_id, track) in &state.tracks {
            match &track.inner {
                cubedaw_lib::TrackInner::Group(inner) => {
                    if let Some(worker_track) = self.group_tracks.get_mut(track_id) {
                        // TODO only do this when the patch is mutated
                        worker_track.sync_with(track, inner, worker_options);
                    } else {
                        dbg!(&track.patch);
                        self.group_tracks.insert(
                            track_id,
                            crate::WorkerGroupTrackState::from_patch(track, inner, worker_options)
                                .unwrap(),
                        )
                    }
                }
                cubedaw_lib::TrackInner::Section(inner) => {
                    if !self.section_tracks.has(track_id) {
                        dbg!(&track.patch);
                        self.section_tracks.insert(
                            track_id,
                            crate::WorkerSectionTrackState::from_patch(
                                track,
                                inner,
                                worker_options,
                            )
                            .unwrap(),
                        )
                    }
                }
            }
        }
        if let Some(a) = self.group_tracks.iter().next() {
            a.1.nodes.get_node(a.1.nodes.input_node().unwrap()).unwrap();
        }
    }
}

#[derive(Debug)]
pub struct WorkerSectionTrackState {
    pub track_nodes: ProcessedNodeGraph,
    pub note_nodes: ProcessedNodeGraph,

    pub notes: IdMap<Note, WorkerNoteState>,
    pub live_notes: IdMap<Note, WorkerLiveNoteState>,
}
impl WorkerSectionTrackState {
    pub fn from_patch(
        track: &Track,
        section_track: &SectionTrack,
        options: &WorkerOptions,
    ) -> Option<Self> {
        let mut this = WorkerSectionTrackState {
            track_nodes: ProcessedNodeGraph::empty(None, Id::invalid()),
            note_nodes: ProcessedNodeGraph::empty(None, Id::invalid()),

            notes: Default::default(),
            live_notes: Default::default(),
        };
        if !this.sync_with(track, section_track, options) {
            return None;
        }
        Some(this)
    }

    pub fn sync_with(
        &mut self,
        track: &Track,
        section_track: &SectionTrack,
        options: &WorkerOptions,
    ) -> bool {
        let patch = &track.patch;
        patch.debug_assert_valid();

        let mut track_output = None;
        let mut note_output = None;

        for (id, node) in patch.nodes() {
            if node.tag() == cubedaw_lib::NodeTag::Special {
                let res = options.registry.get_resource_key_of(&*node.data.inner);
                if res == Id::new("builtin:track_output") {
                    assert!(
                        track_output.replace(id).is_none(),
                        "TODO handle multiple track outputs"
                    );
                } else if res == Id::new("builtin:note_output") {
                    assert!(
                        note_output.replace(id).is_none(),
                        "TODO handle multiple note outputs"
                    );
                } else {
                    todo!()
                }
            }
        }

        let (Some(track_output), Some(note_output)) = (track_output, note_output) else {
            return false;
        };

        self.track_nodes
            .sync_with(patch, options, Some(note_output), track_output);
        self.note_nodes.sync_with(patch, options, None, note_output);

        true
    }

    pub fn empty(track: &Track, section_track: &SectionTrack, options: &WorkerOptions) -> Self {
        use cubedaw_lib::{NodeData, ResourceKey};

        let mut fake_patch = Patch::new();
        let mut insert_node =
            |key: Id<ResourceKey>, num_inputs: u32, num_outputs: u32| -> Id<NodeEntry> {
                let id = Id::arbitrary();
                fake_patch.insert_node(
                    id,
                    NodeData::new_disconnected(
                        key,
                        options.registry.create_state(key, Default::default()),
                    ),
                    vec![1.0; num_inputs as usize],
                    num_outputs,
                );
                id
            };
        let input = insert_node(Id::new(ResourceKey::new("builtin:note_output")), 1, 1);
        let output = insert_node(Id::new(ResourceKey::new("builtin:track_output")), 1, 0);
        fake_patch.insert_cable(
            Id::arbitrary(),
            cubedaw_lib::Cable::new(input, 0, output, 0),
        );

        let fake_track = Track {
            patch: fake_patch,
            // not actually used.
            inner: TrackInner::Section(SectionTrack::new()),

            ..*track
        };

        Self::from_patch(&fake_track, section_track, options).expect("AHHHHHHHHHHHH")
    }
}

#[derive(Debug)]
pub struct WorkerNoteState {
    pub section_id: Id<Section>,
    pub nodes: ProcessedNodeGraph,
}
impl WorkerNoteState {
    pub fn new(section_id: Id<Section>, nodes: ProcessedNodeGraph) -> Self {
        Self { section_id, nodes }
    }
}

#[derive(Debug)]
pub struct WorkerLiveNoteState {
    pub start_pos: i64,
    pub note: Note,
    pub nodes: ProcessedNodeGraph,
    pub samples_elapsed: u64,
}

#[derive(Debug)]
pub struct WorkerGroupTrackState {
    pub nodes: ProcessedNodeGraph,
}

impl WorkerGroupTrackState {
    pub fn from_patch(
        track: &Track,
        group_track: &GroupTrack,
        options: &WorkerOptions,
    ) -> Option<Self> {
        let mut this = Self {
            nodes: ProcessedNodeGraph::empty(None, Id::invalid()),
        };
        if !this.sync_with(track, group_track, options) {
            return None;
        }
        Some(this)
    }

    pub fn sync_with(
        &mut self,
        track: &Track,
        group_track: &GroupTrack,
        options: &WorkerOptions,
    ) -> bool {
        let patch = &track.patch;
        patch.debug_assert_valid();

        // if patch.cables().next().is_none() {
        //     panic!();
        // }

        let mut track_output = None;
        let mut track_input = None;

        for (id, node) in patch.nodes() {
            if node.tag() == cubedaw_lib::NodeTag::Special {
                let res = options.registry.get_resource_key_of(&*node.data.inner);
                if res == Id::new("builtin:track_output") {
                    assert!(
                        track_output.replace(id).is_none(),
                        "TODO handle multiple track outputs"
                    );
                } else if res == Id::new("builtin:track_input") {
                    assert!(
                        track_input.replace(id).is_none(),
                        "TODO handle multiple track inputs"
                    );
                } else {
                    todo!()
                }
            }
        }

        let (Some(track_output), Some(track_input)) = (track_output, track_input) else {
            return false;
        };

        self.nodes
            .sync_with(patch, options, Some(track_input), track_output);

        true
    }

    pub fn empty(track: &Track, group_track: &GroupTrack, options: &WorkerOptions) -> Self {
        use cubedaw_lib::{NodeData, ResourceKey};

        let mut fake_patch = Patch::new();
        let mut insert_node =
            |key: Id<ResourceKey>, num_inputs: u32, num_outputs: u32| -> Id<NodeEntry> {
                let id = Id::arbitrary();
                fake_patch.insert_node(
                    id,
                    NodeData::new_disconnected(
                        key,
                        options.registry.create_state(key, Default::default()),
                    ),
                    vec![1.0; num_inputs as usize],
                    num_outputs,
                );
                id
            };
        let input = insert_node(Id::new(ResourceKey::new("builtin:track_input")), 0, 1);
        let output = insert_node(Id::new(ResourceKey::new("builtin:track_output")), 1, 0);
        fake_patch.insert_cable(
            Id::arbitrary(),
            cubedaw_lib::Cable::new(input, 0, output, 0),
        );

        let fake_track = Track {
            patch: fake_patch,
            // not actually used.
            inner: TrackInner::Group(GroupTrack::new()),

            ..*track
        };

        Self::from_patch(&fake_track, group_track, options).expect("AHHHHHHHHHHHH")
    }
}

#[cfg(test)]
mod tests {
    use cubedaw_lib::{Patch, Track};

    use crate::WorkerOptions;

    use super::{WorkerGroupTrackState, WorkerSectionTrackState};

    #[test]
    fn test_empty_functions() {
        let options = WorkerOptions::default();

        {
            let track = Track::new_section(Patch::new());
            WorkerSectionTrackState::empty(&track, track.inner.section().unwrap(), &options);
        }
        {
            let track = Track::new_group(Patch::new());
            WorkerGroupTrackState::empty(&track, track.inner.group().unwrap(), &options);
        }
    }
}

// TODO
// #[derive(Debug, Clone)]
// pub struct IdVecMap<K: 'static, T = K> {
//     inner: Box<[Option<(Id<K>, T)>]>,
// }
// impl<K: 'static, T> IdVecMap<K, T> {
//     pub fn new(capacity: usize) -> Self {
//         Self {
//             inner: {
//                 let mut vec = Vec::with_capacity(capacity);
//                 vec.resize_with(capacity, || None);
//                 vec.into_boxed_slice()
//             },
//         }
//     }

//     fn find_empty_slot(&mut self) -> Option<&mut Option<(Id<K>, T)>> {
//         self.inner.iter_mut().find(|o| o.is_none())
//     }
//     fn find_slot(&mut self, id: Id<K>) -> Option<&mut Option<(Id<K>, T)>> {
//         self.inner
//             .iter_mut()
//             .find(|o| matches!(o, Some((slot_id, _)) if *slot_id == id))
//     }

//     pub fn insert(&mut self, id: Id<K>, val: T) -> Result<(), T> {
//         match self.find_empty_slot() {
//             Some(slot) => {
//                 *slot = Some((id, val));
//                 Ok(())
//             }
//             None => Err(val),
//         }
//     }

//     pub fn remove(&mut self, id: Id<K>) -> Option<T> {
//         self.find_slot(id).map(|slot| slot.take().unwrap().1)
//     }
// }

// impl<K, V> IntoIterator for IdVecMap<K, T> {
//     type Item = ;
// }
