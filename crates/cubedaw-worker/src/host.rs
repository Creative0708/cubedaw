use std::{sync::Arc, thread};

use cubedaw_lib::{Range, State};
use cubedaw_workerlib::{PreciseSongPos, WorkerJob, WorkerOptions, WorkerState};

use crate::{
    common::{HostToWorkerEvent, WorkerStatus, WorkerToHostEvent},
    worker,
};

/// An interface for controlling the worker threads. This won't run on the ui thread. Please do not run this on the ui thread.
#[derive(Debug)]
pub struct WorkerHost {
    state: State,
    worker_handles: Box<[WorkerHandle]>,
    worker_options: WorkerOptions,
    worker_state: WorkerState,
    worker_tx: crossbeam_channel::Sender<WorkerToHostEvent>, // keep a cloneable reference to send to workers
    rx: crossbeam_channel::Receiver<WorkerToHostEvent>,

    work_tx: crossbeam_channel::Sender<WorkerJob>,
    work_rx: crossbeam_channel::Receiver<WorkerJob>,
}

impl WorkerHost {
    pub fn new(num_workers: usize, state: State, worker_options: WorkerOptions) -> Self {
        assert!(num_workers > 0);

        let (worker_tx, rx) = crossbeam_channel::unbounded();
        let (work_tx, work_rx) = crossbeam_channel::unbounded();

        let mut worker_handles = Vec::with_capacity(num_workers);
        for i in 0..num_workers {
            let handle = WorkerHandle::new(
                i,
                worker_tx.clone(),
                worker_options.clone(),
                work_tx.clone(),
                work_rx.clone(),
            );
            worker_handles.push(handle);
        }

        Self {
            worker_state: WorkerState::new(&state, &worker_options),
            state,
            worker_handles: worker_handles.into_boxed_slice(),
            worker_options,

            rx,
            worker_tx,

            work_rx,
            work_tx,
        }
    }

    pub fn process(&mut self, start_pos: PreciseSongPos) -> PreciseSongPos {
        let Self {
            state,
            worker_handles,
            worker_options,
            worker_state,

            rx,
            worker_tx,

            work_rx,
            work_tx,
        } = self;

        let end_pos = start_pos
            + PreciseSongPos::from_song_pos_f32({
                // samples / (samples/second) / (60 seconds/minute) * beats/minute * units/beat
                worker_options.buffer_size as f32 / worker_options.sample_rate as f32 / 60.0
                    * state.bpm
                    * Range::UNITS_PER_BEAT as f32
            });
        {
            let song_range_that_we_will_process =
                Range::new(start_pos.song_pos, end_pos.ceil_to_song_pos());
            // add jobs to queue
            for (&track_id, _) in &worker_state.section_tracks {
                let track_data = state.tracks.force_get(track_id).inner.section().unwrap();
                for (section_range, section_id) in
                    track_data.sections_intersecting(song_range_that_we_will_process)
                {
                    let section = track_data.section(section_id).unwrap();
                    for note in section.notes_intersecting(
                        section_range.intersect(song_range_that_we_will_process),
                    ) {}
                }
            }
        }

        replace_with::replace_with_or_default(state, |state| {
            let mut state = Arc::new(state);

            for handle in worker_handles.iter_mut() {
                handle
                    .tx
                    .send(HostToWorkerEvent::StartProcessing {
                        state: state.clone(),
                        start_pos,
                    })
                    .expect("worker closed channel??");
            }

            loop {
                match Arc::try_unwrap(state) {
                    Ok(state) => break state,
                    Err(state_) => state = state_,
                };
                let event = rx.recv().expect("channel is disconnected?!??!");
                match event {
                    WorkerToHostEvent::DoneProcessing(finished_job) => {
                        worker_state.return_finished_work(finished_job);
                    }
                }
            }
        });

        end_pos
    }

    pub fn join(self) {
        let WorkerHost {
            state: _,
            worker_handles,
            worker_options: _,
            worker_state: _,

            worker_tx: _,
            rx: _,

            work_tx: _,
            work_rx: _,
        } = self;

        let mut join_handles = Vec::with_capacity(worker_handles.len());
        for worker_handle in worker_handles.into_vec() {
            let WorkerHandle {
                index: _,
                tx,
                join_handle,
            } = worker_handle;

            drop(tx);
            join_handles.push(join_handle);
        }
        for join_handle in join_handles {
            if let Err(err) = join_handle.join() {
                log::warn!("error during thread join: {err:?}");
            }
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

#[derive(Debug)]
struct WorkerHandle {
    pub index: usize,
    pub tx: crossbeam_channel::Sender<HostToWorkerEvent>,
    pub join_handle: thread::JoinHandle<()>,
}
impl WorkerHandle {
    pub fn new(
        index: usize,

        worker_tx: crossbeam_channel::Sender<WorkerToHostEvent>,
        worker_options: WorkerOptions,

        work_tx: crossbeam_channel::Sender<WorkerJob>,
        work_rx: crossbeam_channel::Receiver<WorkerJob>,
    ) -> Self {
        let (tx, worker_rx) = crossbeam_channel::unbounded();
        Self {
            index,
            tx,
            join_handle: thread::Builder::new()
                .name(format!("Audio Worker #{index}"))
                .spawn(move || {
                    worker::run_forever(worker_tx, worker_rx, work_tx, work_rx, worker_options)
                })
                .expect("failed to spawn thread"),
        }
    }
}
