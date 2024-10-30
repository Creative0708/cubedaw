use cubedaw_lib::{Id, PreciseSongPos, State};

pub enum HostToWorkerEvent {
    StartProcessing {
        state: &'static State,
        start_pos: PreciseSongPos,
    },
}

pub enum WorkerToHostEvent {
    /// A job has ended producing audio and is no longer needed.
    FinishJob(JobDescriptor),
    /// Used for synchronization purposes.
    /// Workers must guarantee that they have dropped all references to the state/worker state before sending `WorkerToHostEvent::Idle`.
    Idle,
}
pub enum JobDescriptor {
    NoteProcess {
        track_id: Id<cubedaw_lib::Track>,
        note_descriptor: crate::NoteDescriptor,
    },
    TrackProcess {
        track_id: Id<cubedaw_lib::Track>,
    },
    TrackGroup {
        track_id: Id<cubedaw_lib::Track>,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkerStatus {
    Processing,
    Idle,
}
