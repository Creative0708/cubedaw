use cubedaw_lib::{Id, IdMap, Note, Patch, Section, State, Track};

use crate::{node_graph::ProcessedNodeGraph, WorkerOptions};

#[derive(Debug)]
pub struct WorkerState {
    pub section_tracks: IdMap<Track, WorkerSectionTrackState>,
}

impl WorkerState {
    pub fn new(state: &State, options: &WorkerOptions) -> Self {
        let mut track_map = IdMap::new();
        for (&track_id, track) in &state.tracks {
            let mut node_map = IdMap::new();
            for (node_id, node) in track.patch.nodes() {
                node_map.insert(node_id, options.registry.create_node(node.key));
            }
            track_map.insert(
                track_id,
                WorkerSectionTrackState::from_patch(&track.patch, options),
            );
        }
        Self {
            section_tracks: track_map,
        }
    }
}

pub enum WorkerJob {
    /// Process a single note on a track.
    /// The note can reference an existing note in a `State` or a "live" note not attached to a `State`. See [`NoteDescriptor`] for more details.
    NoteProcess {
        track_id: Id<Track>,
        note_descriptor: NoteDescriptor,
        state: &'static mut WorkerNoteState,
    },
    /// Process a track.
    TrackProcess {
        track_id: Id<Track>,
        state: &'static mut WorkerSectionTrackState,
    },
    TrackGroup {
        track_id: Id<Track>,
    },
    /// Not actually a job. This is an indicator to the worker that they should drop all resources
    Finalize,
}
impl WorkerJob {
    /// Processes the job.
    pub fn process(self, project_state: &State) -> bool {
        match self {
            Self::NoteProcess {
                track_id,
                note_descriptor,
                state,
            } => {
                let possibly_deleted_note = match note_descriptor {
                    NoteDescriptor::State {
                        section_id,
                        note_id,
                    } => project_state
                        .tracks
                        .get(track_id)
                        .and_then(|track| track.inner.section())
                        .and_then(|section_track| section_track.section(section_id))
                        .and_then(|section| section.note(note_id))
                        .map(|(_, note)| note),
                    NoteDescriptor::Live { note, .. } => Some(note),
                };
                let Some(note) = possibly_deleted_note else {
                    return true;
                };

                true
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

#[derive(Debug, Default)]
pub struct WorkerSectionTrackState {
    pub track_nodes: ProcessedNodeGraph,
    pub note_nodes: ProcessedNodeGraph,

    pub notes: IdMap<Note, WorkerNoteState>,
    pub live_notes: IdMap<Note, (Note, WorkerNoteState)>,
}
impl WorkerSectionTrackState {
    pub fn from_patch(patch: &Patch, options: &WorkerOptions) -> Self {
        patch.debug_assert_valid();

        let mut track_output = None;
        let mut note_output = None;

        for (id, node) in patch.nodes() {
            if node.tag == cubedaw_lib::NodeTag::Special {
                let res = options.registry.get_resource_key_of(&*node.inner);
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
                }
            }
        }

        let (Some(track_output), Some(note_output)) = (track_output, note_output) else {
            // give up
            return Default::default();
        };

        Self {
            track_nodes: ProcessedNodeGraph::new(patch, options, Some(note_output), track_output),
            note_nodes: ProcessedNodeGraph::new(patch, options, None, note_output),

            notes: IdMap::new(),
            live_notes: IdMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct WorkerNoteState {
    pub nodes: ProcessedNodeGraph,
}

/// A descriptor for a [`cubedaw_lib::Note`]. Either a path to a note in the State, or a "live"
/// note not attached to the state.
#[derive(Copy, Clone, Debug)]
pub enum NoteDescriptor {
    State {
        section_id: Id<Section>,
        note_id: Id<Note>,
    },
    Live {
        note_id: Id<Note>,
        note: &'static Note,
    },
}
