use std::sync::Arc;

use cubedaw_lib::State;
use cubedaw_workerlib::{NoteDescriptor, PreciseSongPos};

pub enum HostToWorkerEvent {
    StartProcessing {
        state: &'static State,
        start_pos: PreciseSongPos,
    },
}

pub enum WorkerToHostEvent {
    /// A note has ended producing audio and is no longer needed.
    DeleteNoteProcessJob {
        track_id: cubedaw_lib::Id<cubedaw_lib::Track>,
        note_descriptor: NoteDescriptor,
    },
    /// Used for synchronization purposes.
    /// Workers must guarantee that they have dropped all references to the state/worker state before sending `WorkerToHostEvent::Idle`.
    Idle,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkerStatus {
    Processing,
    Idle,
}
