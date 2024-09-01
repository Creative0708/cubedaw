use std::borrow::BorrowMut;

use cubedaw_lib::{builtin_nodes, Buffer, Id, Note, Track};

use crate::{sync, worker::WorkerScratch, ProcessedNodeGraph};

#[derive(Debug)]
pub enum WorkerJob {
    /// Process a single note on a track.
    /// The note can reference an existing note in a `State` or a "live" note not attached to a `State`. See [`NoteDescriptor`] for more details.
    NoteProcess {
        track_id: Id<Track>,
        note_descriptor: NoteDescriptor,
        nodes: &'static mut ProcessedNodeGraph,
        output: sync::SyncAccessibleWriteHandle<'static, Buffer<'static>, WorkerJob>,
    },
    /// Process a track.
    TrackProcess {
        track_id: Id<Track>,
        nodes: &'static mut ProcessedNodeGraph,
        input: sync::SyncAccessibleReadHandle<'static, Buffer<'static>, WorkerJob>,
        output: sync::SyncAccessibleWriteHandle<'static, Buffer<'static>, WorkerJob>,
    },
    TrackGroup {
        track_id: Id<Track>,
        nodes: &'static mut ProcessedNodeGraph,
        input: sync::SyncAccessibleReadHandle<'static, Buffer<'static>, WorkerJob>,
        output: sync::SyncAccessibleWriteHandle<'static, Buffer<'static>, WorkerJob>,
    },
    /// Not actually a job. This is a signal to the worker that they should drop all resources and send the `Idle` event.
    Finalize,
}
impl WorkerJob {
    /// Processes the job.
    pub fn process(
        self,
        state: &cubedaw_lib::State,
        worker_options: &crate::WorkerOptions,
        scratch: &mut WorkerScratch,
    ) -> WorkerJobResult {
        match self {
            Self::NoteProcess {
                track_id,
                note_descriptor,
                nodes,
                output,
            } => {
                let possibly_deleted_note = match note_descriptor {
                    NoteDescriptor::State { note, .. } => note,
                    NoteDescriptor::Live { note, .. } => note,
                };
                // state.nodes

                WorkerJobResult {
                    finished_job_descriptor: None,
                    job_to_add: todo!(),
                }
            }
            Self::TrackProcess {
                track_id,
                nodes,
                input,
                output,
            } => {
                replace_with::replace_with_or_default(scratch, |mut scratch| {
                    let input_node: &mut builtin_nodes::NoteOutputNode = nodes
                        .get_node_mut(nodes.input_node().unwrap())
                        .unwrap()
                        .inner
                        .downcast_mut()
                        .unwrap();
                    input_node.start(builtin_nodes::NoteOutputNodeInner::Input(
                        input.wait().as_slice(),
                    ));
                    let output_node: &mut builtin_nodes::TrackOutputNode = nodes
                        .get_node_mut(nodes.output_node())
                        .unwrap()
                        .inner
                        .downcast_mut()
                        .unwrap();

                    output_node.start(scratch.0);

                    nodes.process(worker_options);

                    let input_node: &mut builtin_nodes::NoteOutputNode = nodes
                        .get_node_mut(nodes.input_node().unwrap())
                        .unwrap()
                        .inner
                        .downcast_mut()
                        .unwrap();
                    input_node.end();
                    let output_node: &mut builtin_nodes::TrackOutputNode = nodes
                        .get_node_mut(nodes.output_node())
                        .unwrap()
                        .inner
                        .downcast_mut()
                        .unwrap();

                    scratch.0 = output_node.end();

                    scratch
                });

                let job_to_add = output.lock(|output_buf| {
                    output_buf.accumulate(&scratch.0.borrow_mut());
                });

                WorkerJobResult {
                    finished_job_descriptor: None,
                    job_to_add,
                }
            }
            Self::TrackGroup {
                track_id,
                nodes,
                input,
                output,
            } => {
                todo!();
            }
            Self::Finalize => unimplemented!("can't call process() on WorkerJob::Finalize"),
        }
    }
    // pub fn track_id(&self) -> Id<Track> {
    //     match *self {
    //         Self::NoteProcess { track_id } => track_id,
    //         Self::TrackProcess { track_id } => track_id,
    //         Self::TrackGroup { track_id } => track_id,
    //     }
    // }
}

pub struct WorkerJobResult {
    /// If the associated job can no longer produce audio, this is `Some(job_descriptor)`. Otherwise, it's `None`.
    pub finished_job_descriptor: Option<crate::common::JobDescriptor>,
    pub job_to_add: Option<WorkerJob>,
}

/// A descriptor for a [`cubedaw_lib::Note`]. Either a path to a note in the State, or a "live"
/// note not attached to the state.
#[derive(Copy, Clone, Debug)]
pub enum NoteDescriptor {
    State {
        note_id: Id<Note>,

        start_pos: i64,
        note: &'static Note,
    },
    Live {
        note_id: Id<Note>,

        start_pos: i64,
        note: &'static Note,
        samples_elapsed: u64,
    },
}
