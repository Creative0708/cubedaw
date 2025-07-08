use anyhow::Result;
use cubedaw_lib::{Buffer, Clip, Id, IdMap, Node, Note, Patch, State, Track};

use crate::{
    WorkerOptions,
    node_graph::{NoteNodeGraph, TrackNodeGraph},
};

#[derive(Debug)]
/// State for the worker host. Parts of these are sent to workers during normal processing.
///
/// This is in addition to a `crate::WorkerState` that each worker has and a `cubedaw_lib::State` that's shared across all workers.
pub struct WorkerHostState {
    pub tracks: IdMap<Track, WorkerTrackState>,
}

impl WorkerHostState {
    pub fn new(state: &State, options: &WorkerOptions) -> Self {
        let mut tracks = IdMap::new();
        for (track_id, track) in &state.tracks {
            let mut node_map = IdMap::new();
            for (node_id, node) in track.patch.nodes() {
                let entry = options.registry.get(&node.data.key).unwrap_or_else(|| {
                    panic!(
                        "key {:?} doesn't exist in registry {:?}",
                        &node.data.key, &options.registry
                    )
                });
                node_map.insert(node_id, (entry.node_factory)(node.data.inner.as_bytes()));
            }
            tracks.insert(
                track_id,
                WorkerTrackState::from_track(track, options)
                    .unwrap_or_else(|_| WorkerTrackState::empty(options)),
            );
        }

        Self { tracks }
    }

    pub fn sync_with(
        &mut self,
        state: &State,
        worker_options: &WorkerOptions,
    ) -> anyhow::Result<()> {
        let mut tracks_to_delete = Vec::new();
        let mut notes_to_delete = Vec::new();
        for (track_id, worker_track_data) in &mut self.tracks {
            match state.tracks.get(track_id) {
                Some(track) => {
                    for (note_id, WorkerNoteState { clip_id, .. }) in &worker_track_data.notes {
                        if track
                            .clip(*clip_id)
                            .and_then(|clip| clip.note(note_id))
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
            self.tracks.remove(track_id);
        }

        for (track_id, track) in &state.tracks {
            // match &track.inner {
            //     cubedaw_lib::TrackInner::Group(inner) => {
            //         if let Some(worker_track) = self.group_tracks.get_mut(track_id) {
            //             // TODO only do this when the patch is mutated
            //             worker_track.sync_with(track, worker_options)?;
            //         } else {
            //             dbg!(&track.patch);
            //             self.group_tracks.insert(
            //                 track_id,
            //                 WorkerGroupTrackState::from_patch(track, worker_options)
            //                     .unwrap(),
            //             )
            //         }
            //     }
            if let Some(worker_track) = self.tracks.get_mut(track_id) {
                // TODO only do this when the patch is mutated
                worker_track.sync_with(track, worker_options)?;
            } else {
                dbg!(&track.patch);
                self.tracks.insert(
                    track_id,
                    WorkerTrackState::from_track(track, worker_options).unwrap(),
                )
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct WorkerTrackState {
    pub track_nodes: TrackNodeGraph,
    pub note_nodes: NoteNodeGraph,

    // TODO: switch these to vec for optimization purposes (when necessary)
    pub notes: IdMap<Note, WorkerNoteState>,
    pub live_notes: IdMap<Note, WorkerLiveNoteState>,
}
impl WorkerTrackState {
    pub fn from_track(track: &Track, options: &WorkerOptions) -> anyhow::Result<Self> {
        let mut this = WorkerTrackState {
            track_nodes: TrackNodeGraph::empty(),
            note_nodes: NoteNodeGraph::empty(),

            notes: Default::default(),
            live_notes: Default::default(),
        };
        this.sync_with(track, options)?;
        Ok(this)
    }

    pub fn sync_with(&mut self, track: &Track, options: &WorkerOptions) -> anyhow::Result<()> {
        let patch = &track.patch;
        patch.debug_assert_valid();

        self.track_nodes.sync_with(patch, options)?;
        self.note_nodes.sync_with(patch, options)?;

        // hella inefficient bc we're doing an unnecessary topo sort every time but i cannot be bothered (yet)
        for (_note_id, note_state) in &mut self.notes {
            note_state.sync_with(track, options)?;
        }
        for (_note_id, note_state) in &mut self.live_notes {
            note_state.sync_with(track, options)?;
        }

        Ok(())
    }

    pub fn empty(options: &WorkerOptions) -> Self {
        use cubedaw_lib::{NodeData, ResourceKey};

        let mut fake_patch = Patch::new();
        let mut insert_node =
            |key: ResourceKey, num_inputs: u32, num_outputs: u32, inner: Box<Buffer>| -> Id<Node> {
                let id = Id::arbitrary();
                fake_patch.insert_node(
                    id,
                    NodeData::new_disconnected(key, inner),
                    vec![1.0; num_inputs as usize],
                    num_outputs,
                );
                id
            };
        let input = insert_node(
            resourcekey::literal!("builtin:output"),
            1,
            1,
            Default::default(),
        );
        let output = insert_node(
            resourcekey::literal!("builtin:track_output"),
            1,
            0,
            Default::default(),
        );
        fake_patch.insert_cable(
            Id::arbitrary(),
            cubedaw_lib::Cable::new(input, 0, output, 0, 0),
            cubedaw_lib::CableConnection { multiplier: 1.0 },
        );

        // ideally we'd construct a `Self` directly instead of using this function but whatever. TODO
        let fake_track = Track::new(fake_patch);

        Self::from_track(&fake_track, options).expect("failed to construct an empty patch??")
    }
}

#[derive(Debug)]
pub struct WorkerNoteState {
    pub clip_id: Id<Clip>,
    pub nodes: NoteNodeGraph,
}
impl WorkerNoteState {
    pub fn sync_with(&mut self, track: &Track, options: &WorkerOptions) -> Result<()> {
        self.nodes.sync_with(&track.patch, options)
    }
}

#[derive(Debug)]
pub struct WorkerLiveNoteState {
    pub start_pos: i64,
    pub note: Note,
    pub nodes: NoteNodeGraph,
    pub samples_elapsed: u64,
}
impl WorkerLiveNoteState {
    pub fn sync_with(&mut self, track: &Track, options: &WorkerOptions) -> Result<()> {
        self.nodes.sync_with(&track.patch, options)
    }
}

#[cfg(test)]
mod tests {

    use crate::WorkerOptions;

    use super::WorkerTrackState;

    #[test]
    fn test_empty() {
        let options = WorkerOptions::new(Default::default());

        WorkerTrackState::empty(&options);
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
