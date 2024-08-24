use std::sync::Arc;

use cubedaw_lib::State;
use cubedaw_workerlib::{PreciseSongPos, WorkerJob, WorkerJobResult};

pub enum HostToWorkerEvent {
    StartProcessing {
        state: Arc<State>,
        start_pos: PreciseSongPos,
    },
}

pub enum WorkerToHostEvent {
    DoneProcessing(WorkerJobResult),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WorkerStatus {
    Processing,
    Idle,
}
