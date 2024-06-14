use std::sync::mpsc;

use cubedaw_lib::State;
use cubedaw_workerlib::{WorkerJob, WorkerOptions};

use crate::common::{HostToWorkerEvent, WorkerToHostEvent};

pub fn run_forever(tx: mpsc::Sender<WorkerToHostEvent>, rx: mpsc::Receiver<HostToWorkerEvent>) {
    let mut worker_options = None;
    while let Ok(event) = rx.recv() {
        match event {
            HostToWorkerEvent::Options(options) => {
                worker_options = Some(options);
            }
            HostToWorkerEvent::StartProcessing {
                state,
                work,
                start_pos,
            } => {
                while let Some(job) = work.pop() {
                    let buf = process_job(&job, &state, worker_options.as_ref().expect("HostToWorkerEvent::StartProcessing called before HostToWorkerEvent::Options"));
                    tx.send(WorkerToHostEvent::DoneProcessing {
                        finished_buf: buf,
                        finished_job: job,
                    })
                    .expect("channel to host closed??");
                }
                drop(state);
                tx.send(WorkerToHostEvent::Idle)
                    .expect("channel to host closed??");
            }
        }
    }
}

pub fn process_job(job: &WorkerJob, state: &State, options: &WorkerOptions) -> Box<[f32]> {
    todo!()
}
