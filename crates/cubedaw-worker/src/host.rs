use std::{
    sync::{mpsc, Arc},
    thread,
};

use cubedaw_lib::State;
use cubedaw_workerlib::{SamplePos, WorkerOptions, WorkerState};

use crate::{
    common::{HostToWorkerEvent, WorkerQueue, WorkerStatus, WorkerToHostEvent},
    worker,
};

/// An interface for controlling the worker threads. This won't run on the ui thread
pub struct WorkerHost {
    inner: WorkerHostInner,

    worker_state: WorkerState,
    worker_handles: Box<[WorkerHandle]>,

    rx: mpsc::Receiver<WorkerToHostEvent>,
    worker_tx: mpsc::Sender<WorkerToHostEvent>, // keep a cloneable reference to send to workers

    work_queue: Arc<WorkerQueue>,

    options: WorkerOptions,
}

impl WorkerHost {
    pub fn new(state: State, options: WorkerOptions) -> Self {
        let (worker_tx, rx) = mpsc::channel();
        Self {
            worker_state: WorkerState::new(&state, &options),
            worker_handles: Box::new([]),

            rx,
            worker_tx,

            inner: WorkerHostInner::Idle { state },

            work_queue: Arc::new(WorkerQueue::new()),

            options,
        }
    }

    pub fn init_workers(&mut self, num_workers: usize) {
        assert!(num_workers > 0);

        let _ = self.collect();

        if self.worker_handles.len() == num_workers {
            return;
        }

        // doesn't actually allocate. should get optimized out
        let mut vec = core::mem::replace(&mut self.worker_handles, Box::new([])).into_vec();

        if num_workers > vec.len() {
            vec.reserve_exact(num_workers);
            for i in vec.len() + 1..=num_workers {
                let handle = WorkerHandle::new(i, self.worker_tx.clone());
                handle
                    .tx
                    .send(HostToWorkerEvent::Options(self.options.clone()))
                    .expect("i literally just made the channel how is it closed");
                vec.push(handle);
            }
        }

        self.worker_handles = vec.into_boxed_slice();
    }

    pub fn collect(&mut self) -> &mut State {
        loop {
            if let WorkerHostInner::Idle { ref mut state } = self.inner {
                return state;
            }
            match self.rx.recv().expect("channel is disconnected?!??!") {
                WorkerToHostEvent::DoneProcessing {
                    finished_buf,
                    finished_job: finished_work,
                } => {
                    // TODO do stuff with finished_buf

                    self.worker_state.return_finished_work(finished_work);
                }
                WorkerToHostEvent::Idle => {
                    // TODO do we really need handle.status to exist?
                    // handle.status = WorkerStatus::Idle;
                }
            }

            let WorkerHostInner::Processing { state } =
                core::mem::replace(&mut self.inner, WorkerHostInner::Dummy)
            else {
                unreachable!();
            };

            // the workers drop the Arc<State> when they finish so when all workers
            // are idle there will only be one strong reference to self.inner.state
            self.inner = match Arc::try_unwrap(state) {
                Ok(state) => WorkerHostInner::Idle { state },
                Err(state) => WorkerHostInner::Processing { state },
            };
        }
    }

    pub fn queue(&mut self, start_pos: SamplePos) {
        self.collect();

        assert!(self.work_queue.is_empty());

        let WorkerHostInner::Idle { state } =
            core::mem::replace(&mut self.inner, WorkerHostInner::Dummy)
        else {
            unreachable!();
        };
        let state = Arc::new(state);

        for handle in self.worker_handles.iter_mut() {
            handle.status = WorkerStatus::Processing;
            handle
                .tx
                .send(HostToWorkerEvent::StartProcessing {
                    state: state.clone(),
                    work: self.work_queue.clone(),
                    start_pos,
                })
                .expect("worker closed channel??");
        }

        self.inner = WorkerHostInner::Processing { state };
    }

    pub fn join(self) {
        // https://github.com/rust-lang/rust/issues/59878
        let worker_handles = self.worker_handles.into_vec();

        let join_handles = worker_handles
            .into_iter()
            .map(|handle| handle.join_handle)
            .collect::<Vec<_>>();

        // handle.tx gets dropped & closed so the workers should exit
        for join_handle in join_handles {
            join_handle.join().expect("worker thread panicked");
        }
    }

    pub fn options(&self) -> &WorkerOptions {
        &self.options
    }

    pub fn set_options(&mut self, options: WorkerOptions) {
        for handle in self.worker_handles.iter() {
            handle
                .tx
                .send(HostToWorkerEvent::Options(options.clone()))
                .expect("worker closed channel??");
        }

        self.options = options;
    }
}

// impl From<io::Error> for WorkerHostError {
//     fn from(value: io::Error) -> Self {
//         Self::Io(value)
//     }
// }

enum WorkerHostInner {
    Idle { state: State },
    Processing { state: Arc<State> },

    // TODO this is used to move out of WorkerHostInner when replacing it with a new value.
    // find a better way to do this?
    // there are crates like take_mut but those come at a runtime cost and also cancel panics
    Dummy,
}

struct WorkerHandle {
    pub index: usize,
    pub tx: mpsc::Sender<HostToWorkerEvent>,
    pub status: WorkerStatus,
    pub join_handle: thread::JoinHandle<()>,
}
impl WorkerHandle {
    pub fn new(index: usize, worker_tx: mpsc::Sender<WorkerToHostEvent>) -> Self {
        let (tx, worker_rx) = mpsc::channel();
        Self {
            index,
            tx,
            status: WorkerStatus::Idle,
            join_handle: thread::Builder::new()
                .name(format!("Audio Worker #{index}"))
                .spawn(move || worker::run_forever(worker_tx, worker_rx))
                .expect("failed to spawn thread"),
        }
    }
}
