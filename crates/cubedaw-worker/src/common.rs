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

    /// The worker encountered an error while processing.
    /// This (unless there are bugs) always means that something has gone wrong that the app can't control; e.g. corrupted project files, abnormal plugin behavior, etc.
    ///
    /// Since errors may leave worker state in a poisoned state, this means that the worker host has to be reloaded.
    Error(anyhow::Error),
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
