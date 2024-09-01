use cubedaw_lib::State;

use crate::{
    common::{HostToWorkerEvent, WorkerToHostEvent},
    WorkerJob,
};

pub fn run_forever(
    tx: crossbeam_channel::Sender<WorkerToHostEvent>,
    rx: crossbeam_channel::Receiver<HostToWorkerEvent>,

    work_tx: crossbeam_channel::Sender<WorkerJob>,
    work_rx: crossbeam_channel::Receiver<WorkerJob>,

    options: WorkerOptions,
) {
    let mut scratch = WorkerScratch::new(&options);

    while let Ok(event) = rx.recv() {
        match event {
            HostToWorkerEvent::StartProcessing { state, start_pos } => {
                loop {
                    match work_rx.recv() {
                        Ok(WorkerJob::Finalize) => break,
                        Ok(job) => {
                            let result = job.process(state, &options, &mut scratch);

                            if let Some(job_descriptor) = result.finished_job_descriptor {
                                tx.send(WorkerToHostEvent::FinishJob(job_descriptor))
                                    .expect("channel closed during processing");
                            }
                            if let Some(job_to_add) = result.job_to_add {
                                match job_to_add {
                                    WorkerJob::Finalize => {
                                        // repeat the finalization signal to all workers
                                        for _ in 0..options.num_workers {
                                            work_tx.send(WorkerJob::Finalize).unwrap();
                                        }
                                    }
                                    job_to_add => {
                                        work_tx.send(job_to_add).unwrap();
                                    }
                                }
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

#[derive(Clone, Debug)]
/// Static worker options. These don't change (unless the worker host is reloaded.)
pub struct WorkerOptions {
    pub registry: std::sync::Arc<cubedaw_lib::NodeRegistry>,

    pub num_workers: u32,

    pub sample_rate: u32,
    pub buffer_size: u32,
}

impl Default for WorkerOptions {
    fn default() -> Self {
        Self {
            registry: Default::default(),

            // num_workers: std::thread::available_parallelism()
            //     .map(std::num::NonZero::get)
            //     .unwrap_or(1)
            //     .try_into()
            //     .unwrap_or(u32::MAX), // just to be safe
            num_workers: 1, // TODO remove

            sample_rate: 44100,
            buffer_size: 256,
        }
    }
}

// two oughta be enough for everyone
#[derive(Debug, Clone, Default)]
#[allow(unused)]
pub struct WorkerScratch(pub cubedaw_lib::BufferOwned, pub cubedaw_lib::BufferOwned);

impl WorkerScratch {
    pub fn new(options: &WorkerOptions) -> Self {
        Self(
            cubedaw_lib::BufferOwned::zeroed(options.buffer_size),
            cubedaw_lib::BufferOwned::zeroed(options.buffer_size),
        )
    }
}
