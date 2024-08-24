use cubedaw_lib::State;
use cubedaw_workerlib::{WorkerJob, WorkerOptions};

use crate::common::{HostToWorkerEvent, WorkerToHostEvent};

pub fn run_forever(
    tx: crossbeam_channel::Sender<WorkerToHostEvent>,
    rx: crossbeam_channel::Receiver<HostToWorkerEvent>,

    work_tx: crossbeam_channel::Sender<WorkerJob>,
    work_rx: crossbeam_channel::Receiver<WorkerJob>,

    options: WorkerOptions,
) {
    while let Ok(event) = rx.recv() {
        match event {
            HostToWorkerEvent::StartProcessing { state, start_pos } => {
                loop {
                    match work_rx.recv() {
                        Ok(WorkerJob::Finalize) => break,
                        Ok(job) => {
                            let result = job.process(&state);
                            tx.send(WorkerToHostEvent::DoneProcessing(result))
                                .expect("channel closed during processing");
                        }
                        Err(crossbeam_channel::RecvError) => {
                            panic!("channel closed during processing");
                        }
                    }
                }
                drop(state);
            }
        }
    }
}

pub fn process_job(job: &WorkerJob, state: &State, options: &WorkerOptions) -> Box<[f32]> {
    todo!()
}
