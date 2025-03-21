use anyhow::Result;
use cubedaw_lib::{Buffer, Id, Note, PreciseSongPos, Track};

use crate::{
    WorkerState,
    common::JobDescriptor,
    node_graph::{NoteNodeGraph, TrackNodeGraph},
    sync,
    worker::WorkerScratch,
};

#[derive(Debug)]
pub enum WorkerJob {
    /// Process a single note on a track.
    /// The note can reference an existing note in a `State` or a "live" note not attached to a `State`. See [`NoteDescriptor`] for more details.
    NoteProcess {
        track_id: Id<Track>,
        note_descriptor: NoteDescriptor,
        nodes: &'static mut NoteNodeGraph,
        output: sync::SyncAccessibleWriteHandle<'static, &'static mut Buffer, WorkerJob>,
    },
    /// Process a track.
    TrackProcess {
        track_id: Id<Track>,
        nodes: &'static mut TrackNodeGraph,
        input: sync::SyncAccessibleReadHandle<'static, &'static mut Buffer, WorkerJob>,
        output: sync::SyncAccessibleWriteHandle<'static, &'static mut Buffer, WorkerJob>,
    },
    /// Not actually a job. This is a signal to the worker that they should drop all resources and send the `Idle` event.
    Finalize,
}
impl WorkerJob {
    /// Processes the job.
    pub fn process(
        self,
        state: &cubedaw_lib::State,
        start_pos: PreciseSongPos,
        worker_options: &crate::WorkerOptions,
        worker_state: &mut WorkerState,
        scratch: &mut WorkerScratch,
    ) -> Result<WorkerJobResult> {
        let _ = (state, start_pos, scratch);
        Ok(match self {
            Self::NoteProcess {
                track_id,
                note_descriptor,
                nodes,
                output,
            } => {
                let note = match note_descriptor {
                    NoteDescriptor::State { note, .. } => note,
                    NoteDescriptor::Live { note, .. } => note,
                };

                // TODO: tail detection and note autoremoval
                let buffer = nodes.process(worker_options, worker_state, note)?;

                let job_to_add = output.lock(|output_buf| {
                    output_buf.accumulate(buffer);
                });

                WorkerJobResult {
                    finished_job_descriptor: Some(JobDescriptor::NoteProcess {
                        track_id,
                        note_descriptor,
                    }),
                    job_to_add,
                }
            }
            Self::TrackProcess {
                track_id,
                nodes,
                input,
                output,
            } => {
                let buffer = nodes.process(worker_options, worker_state, input.wait())?;

                // dbg!(buffer);

                let job_to_add = output.lock(|output_buf| {
                    output_buf.accumulate(buffer);
                });

                WorkerJobResult {
                    finished_job_descriptor: Some(JobDescriptor::TrackProcess { track_id }),
                    job_to_add,
                }
            }
            Self::Finalize => unimplemented!("can't call process() on WorkerJob::Finalize"),
        })
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
// TODO: use these
#[allow(unused)]
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
