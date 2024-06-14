use std::sync::Arc;

use crossbeam::queue::SegQueue;
use cubedaw_lib::State;
use cubedaw_workerlib::{SamplePos, WorkerJob, WorkerOptions};

pub(crate) type WorkerQueue = SegQueue<WorkerJob>;

pub enum HostToWorkerEvent {
    Options(WorkerOptions),
    StartProcessing {
        state: Arc<State>,
        work: Arc<WorkerQueue>,
        start_pos: SamplePos,
    },
}

pub enum WorkerToHostEvent {
    DoneProcessing {
        finished_buf: Box<[f32]>,
        finished_job: WorkerJob,
    },
    Idle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Processing,
    Idle,
}
