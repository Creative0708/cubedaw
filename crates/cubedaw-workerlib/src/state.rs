use cubedaw_lib::{Id, IdMap, NodeData, Note, Patch, Section, State, Track};
use cubedaw_node::DynNode;

use crate::{NodeRegistry, SyncCumulativeBuffer, WorkerOptions};

pub struct WorkerState {
    track: IdMap<Track, WorkerSectionTrackState>,
    track_results: IdMap<Track, SyncCumulativeBuffer>,
}

impl WorkerState {
    pub fn new(state: &State, options: &WorkerOptions) -> Self {
        let mut track_map = IdMap::new();
        let mut track_results = IdMap::new();
        for (&track_id, track) in &state.tracks {
            let mut node_map = IdMap::new();
            for (node_id, node) in track.patch.nodes() {
                node_map.insert(node_id, options.node_registry.create_node(node.key));
            }
            track_map.insert(
                track_id,
                WorkerSectionTrackState::from_patch(&track.patch, &options.node_registry),
            );
            track_results.insert(
                track_id,
                SyncCumulativeBuffer::new(options.buffer_size as _),
            );
        }
        Self {
            track: track_map,
            track_results,
        }
    }

    pub fn return_finished_work(&mut self, work: WorkerJob) {
        match work {
            WorkerJob::NoteProcess {
                track_id,
                note_descriptor,
                is_done,
                state,
            } => {
                if is_done {
                    // note is finished, no need to return it
                    return;
                }

                let Some(track) = self.track.get_mut(track_id) else {
                    // track is deleted, no need to return it
                    return;
                };

                match note_descriptor {
                    NoteDescriptor::State {
                        section_id: _section_id,
                        note_id,
                    } => {
                        track.notes.insert(note_id, state);
                    }
                    NoteDescriptor::Live { note_id, note } => {
                        track.live_notes.insert(note_id, (note, state));
                    }
                }
            }
            _ => todo!(),
        }
    }
}

pub enum WorkerJob {
    NoteProcess {
        track_id: Id<Track>,
        note_descriptor: NoteDescriptor,
        is_done: bool,
        state: WorkerNoteState,
    },
    TrackProcess {
        track_id: Id<Track>,
        state: WorkerSectionTrackState,
    },
    TrackGroup {
        track_id: Id<Track>,
    },
}
impl WorkerJob {
    pub fn process(&mut self, project_state: &State) {
        match *self {
            Self::NoteProcess {
                track_id,
                ref note_descriptor,
                ref mut is_done,
                ref state,
            } => {
                let note = match *note_descriptor {
                    NoteDescriptor::State {
                        section_id,
                        note_id,
                    } => project_state
                        .tracks
                        .get(track_id)
                        .and_then(|track| {
                            track
                                .inner
                                .section()
                                .expect("track isn't a section track???")
                                .section(section_id)
                        })
                        .and_then(|section| section.note(note_id)),
                    NoteDescriptor::Live { ref note, .. } => Some(note),
                };
            }
            _ => todo!(),
        }
    }
    // pub fn track_id(&self) -> Id<Track> {
    //     match self {
    //         Self::NoteProcess { track_id } => *track_id,
    //         Self::TrackProcess { track_id } => *track_id,
    //         Self::TrackGroup { track_id } => *track_id,
    //     }
    // }
}

#[derive(Default)]
pub struct WorkerSectionTrackState {
    pub track_nodes: IdMap<NodeData, DynNode>,
    pub note_nodes: IdMap<NodeData, DynNode>,

    pub notes: IdMap<Note, WorkerNoteState>,
    pub live_notes: IdMap<Note, (Note, WorkerNoteState)>,
}
impl WorkerSectionTrackState {
    pub fn from_patch(patch: &Patch, registry: &NodeRegistry) -> Self {
        patch.debug_assert_valid();

        let mut track_nodes = IdMap::new();
        let mut note_nodes = IdMap::new();

        for (id, note) in patch.nodes() {
            match note.tag {
                cubedaw_lib::NodeTag::Disconnected => (),
                cubedaw_lib::NodeTag::Note => {
                    note_nodes.insert(id, registry.create_node(note.key));
                }
                cubedaw_lib::NodeTag::Track => {
                    track_nodes.insert(id, registry.create_node(note.key));
                }
            }
        }

        Self {
            track_nodes,
            note_nodes,

            notes: IdMap::new(),
            live_notes: IdMap::new(),
        }
    }
}

pub struct WorkerNoteState {
    pub nodes: IdMap<NodeData, DynNode>,
}

/// A descriptor for a [`cubedaw_lib::Note`]. Either a path to a note in the State, or a "live"
/// note not attached to the state.
#[derive(Clone)]
enum NoteDescriptor {
    State {
        section_id: Id<Section>,
        note_id: Id<Note>,
    },
    Live {
        note_id: Id<Note>,
        note: Note,
    },
}
