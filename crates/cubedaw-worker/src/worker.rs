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
                            if let job @ WorkerJob::NoteProcess {
                                track_id,
                                note_descriptor,
                                ..
                            } = job
                            {
                                let is_done = job.process(&state);
                                if is_done {
                                    tx.send(WorkerToHostEvent::DeleteNoteProcessJob {
                                        track_id,
                                        note_descriptor,
                                    })
                                    .expect("channel closed during processing");
                                }
                            } else {
                                // TODO: handle other events
                                job.process(&state);
                            }
                        }
                        Err(crossbeam_channel::RecvError) => {
                            panic!("channel closed during processing");
                        }
                    }
                }
                // Note: as per the documentation of `WorkerToHostEvent::Idle`, workers must send exactly 1 of this event
                // and must have dropped all references given from work_rx. Not doing this will cause UB.
                // Just, like, don't be stupid with how the worker is implemented.
                tx.send(WorkerToHostEvent::Idle);
            }
        }
    }
}

pub fn process_job(job: &WorkerJob, state: &State, options: &WorkerOptions) -> Box<[f32]> {
    todo!()
}
