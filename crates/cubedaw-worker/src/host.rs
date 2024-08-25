use std::{sync::Arc, thread};

use cubedaw_lib::{Range, State};
use cubedaw_workerlib::{PreciseSongPos, WorkerJob, WorkerOptions, WorkerState};

use crate::{
    common::{HostToWorkerEvent, WorkerStatus, WorkerToHostEvent},
    worker,
};

/// An interface for controlling the audio worker threads. This won't run on the ui thread. Please do not run this on the ui thread.
#[derive(Debug)]
pub struct WorkerHost {
    state: State,
    worker_state: WorkerState,

    worker_handles: Box<[WorkerHandle]>,
    worker_options: WorkerOptions,
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

    pub fn process(self, start_pos: Option<&mut PreciseSongPos>) -> Self {
        use std::{cell::UnsafeCell, mem::ManuallyDrop};

        let Self {
            state,
            worker_state,

            mut worker_handles,
            worker_options,

            rx,
            worker_tx,

            work_rx,
            work_tx,
        } = self;

        let mut allocator = Box::leak(Box::new(bumpalo::Bump::new()));
        let state = allocator.alloc(ManuallyDrop::new(UnsafeCell::new(state)));
        let worker_state = allocator.alloc(ManuallyDrop::new(UnsafeCell::new(worker_state)));

        {
            if let Some(&mut start_pos) = start_pos {
                let end_pos = start_pos
                    + PreciseSongPos::from_song_pos_f32({
                        // samples / (samples/second) / (60 seconds/minute) * beats/minute * units/beat
                        worker_options.buffer_size as f32 / worker_options.sample_rate as f32 / 60.0
                            * state.get_mut().bpm
                            * Range::UNITS_PER_BEAT as f32
                    });
                {
                    let song_range_that_we_will_process =
                        Range::new(start_pos.song_pos, end_pos.ceil_to_song_pos());
                    // add jobs to queue
                    for (&track_id, _) in &worker_state.get_mut().section_tracks {
                        let track_data = state
                            .get_mut()
                            .tracks
                            .force_get(track_id)
                            .inner
                            .section()
                            .unwrap();
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
            }
            // SAFETY: `worker_state` is not borrowed at all after the end of this block (until the workers finish ofc)
            let worker_state = unsafe { &mut *worker_state.get() };
            {
                // live jobs
                for (&track_id, track_data) in &mut worker_state.section_tracks {
                    for (&live_note_id, (live_note, note_state)) in &mut track_data.live_notes {
                        work_tx
                            .send(WorkerJob::NoteProcess {
                                track_id,
                                note_descriptor: cubedaw_workerlib::NoteDescriptor::Live {
                                    note_id: live_note_id,
                                    note: live_note,
                                },
                                state: note_state,
                            })
                            .unwrap();
                    }
                }
            }
        }

        let mut deleted_notes = Vec::new();
        {
            for handle in worker_handles.iter_mut() {
                handle
                    .tx
                    .send(HostToWorkerEvent::StartProcessing {
                        // SAFETY: This is a shared immutable borrow of state. No mutations happen until after all the workers are done.
                        state: unsafe { &*state.get() },
                        start_pos: match start_pos {
                            Some(ref start_pos) => **start_pos,
                            None => Default::default(),
                        },
                    })
                    .expect("worker closed channel??");
            }

            let mut remaining_processing_workers = worker_options.num_workers;

            loop {
                let event = rx.recv().expect("channel is disconnected?!??!");
                match event {
                    WorkerToHostEvent::Idle => {
                        remaining_processing_workers -= 1;
                        if remaining_processing_workers == 0 {
                            break;
                        }
                    }
                    WorkerToHostEvent::DeleteNoteProcessJob {
                        track_id,
                        note_descriptor,
                    } => {
                        deleted_notes.push((track_id, note_descriptor));
                    }
                }
            }
        }

        // SAFETY: Several things going on here:
        // - `state` and `worker_state` are shadowed, preventing further use of the `ManuallyDrop` in this thread.
        // - All workers have sent `WorkerToHostEvent::Idle` which means they don't hold any references to `allocator`.
        //   - This makes `ManuallyDrop::take(<state/worker_state>)` safe.
        //   - This makes `Box::from_raw(allocator)` safe.
        let (state, mut worker_state) = unsafe {
            let ret = (
                ManuallyDrop::take(state).into_inner(),
                ManuallyDrop::take(worker_state).into_inner(),
            );

            drop(Box::from_raw(allocator));

            ret
        };

        for (track_id, note_descriptor) in deleted_notes {
            let track = worker_state.section_tracks.force_get_mut(track_id);
            match note_descriptor {
                cubedaw_workerlib::NoteDescriptor::Live { note_id, note: _ } => {
                    track
                        .live_notes
                        .remove(note_id)
                        .expect("tried to remove nonexistent note");
                }
                cubedaw_workerlib::NoteDescriptor::State {
                    section_id: _,
                    note_id,
                } => {
                    track.notes.remove(note_id);
                }
            }
        }

        Self {
            state,
            worker_state,

            worker_handles,
            worker_options,

            rx,
            worker_tx,

            work_rx,
            work_tx,
        }
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
