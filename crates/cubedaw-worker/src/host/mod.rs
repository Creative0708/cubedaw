use std::{fmt::Debug, sync::Arc, thread};

use anyhow::Context;
use cubedaw_lib::{Buffer, IdMap, InternalBufferType, PreciseSongPos, Range, State};
use unwrap_todo::UnwrapTodo;

use crate::{
    WorkerJob, WorkerOptions,
    common::{HostToWorkerEvent, JobDescriptor, WorkerToHostEvent},
    sync::SyncBuffer,
    worker,
};
mod state;
pub use state::{
    WorkerGroupTrackState, WorkerHostState, WorkerLiveNoteState, WorkerNoteState,
    WorkerSectionTrackState,
};

/// An interface for controlling the audio worker threads. This won't run on the ui thread. Please do not run this on the ui thread.
#[derive(Debug)]
pub struct WorkerHost {
    state: State,
    worker_state: WorkerHostState,

    worker_handles: Box<[WorkerHandle]>,
    worker_options: Arc<WorkerOptions>,
    worker_tx: crossbeam_channel::Sender<WorkerToHostEvent>, // keep a cloneable reference to send to workers
    rx: crossbeam_channel::Receiver<WorkerToHostEvent>,

    work_tx: crossbeam_channel::Sender<WorkerJob>,
    work_rx: crossbeam_channel::Receiver<WorkerJob>,
}

impl WorkerHost {
    pub fn new(state: State, worker_options: WorkerOptions) -> Self {
        let worker_options = Arc::new(worker_options);

        let (worker_tx, rx) = crossbeam_channel::unbounded();
        let (work_tx, work_rx) = crossbeam_channel::unbounded();

        let mut worker_handles = Vec::with_capacity(worker_options.num_workers as usize);
        for i in 0..worker_options.num_workers {
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
            worker_state: WorkerHostState::new(&state, &worker_options),
            state,
            worker_handles: worker_handles.into_boxed_slice(),
            worker_options,

            rx,
            worker_tx,

            work_rx,
            work_tx,
        }
    }

    fn sync_with_state(&mut self) {
        let Self {
            state,
            worker_options,
            ..
        } = self;

        self.worker_state
            .sync_with(state, worker_options)
            .with_context(|| format!("state is {state:?}"))
            .todo();
    }

    /// Delete all currently processing jobs. This will result in silence
    pub fn stop_all_processing(&mut self) {
        for track_state in self.worker_state.section_tracks.values_mut() {
            track_state.live_notes.clear();
            track_state.notes.clear();
            track_state.track_nodes.reset();
        }
        for track_state in self.worker_state.group_tracks.values_mut() {
            track_state.nodes.reset();
        }
    }

    pub fn options(&self) -> &WorkerOptions {
        &self.worker_options
    }

    pub fn process(
        mut self,
        mut start_pos: Option<&mut PreciseSongPos>,
        live_pos: PreciseSongPos,

        output: &mut Buffer,
    ) -> Self {
        self.sync_with_state();

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

        let allocator_cell = UnsafeCell::new(Box::leak(Box::new(bumpalo::Bump::new())));
        // SAFETY: allocator_cell isn't touched until after all the workers are finished processing.
        let allocator = unsafe { &**allocator_cell.get() };
        let state = allocator.alloc(ManuallyDrop::new(UnsafeCell::new(state)));
        let worker_state = allocator.alloc(ManuallyDrop::new(UnsafeCell::new(worker_state)));

        let master_output = {
            // SAFETY: `state` and `worker_state` are not borrowed at all after the end of this block (until the workers finish ofc)
            let (state, worker_state) = unsafe { (&*state.get(), &mut *worker_state.get()) };

            add_jobs(
                allocator,
                &work_tx,
                state,
                worker_state,
                &worker_options,
                start_pos.as_deref_mut(),
                live_pos,
            )
        };

        // tell the workers to start with all the infomation they need
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

        let mut deleted_notes = Vec::new();

        // aaaaand we're off! :DDD
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
                WorkerToHostEvent::FinishJob(job_descriptor) => match job_descriptor {
                    JobDescriptor::NoteProcess {
                        track_id,
                        note_descriptor,
                    } => {
                        deleted_notes.push((track_id, note_descriptor));
                    }
                    JobDescriptor::TrackProcess { track_id } => {
                        todo!("{track_id:?}");
                    }
                    JobDescriptor::TrackGroup { track_id } => {
                        todo!("{track_id:?}");
                    }
                },
                WorkerToHostEvent::Error(err) => {
                    // TODO
                    panic!("error encountered in worker host: {err}");
                }
            }
        }

        let final_buffer = master_output
            .try_wait()
            .expect("the jobs are finished; this shouldn't need to block");
        output.copy_from(final_buffer);

        // SAFETY: Several things going on here:
        // - `state` and `worker_state` are shadowed, preventing further use of the `ManuallyDrop`.
        // - All workers have sent `WorkerToHostEvent::Idle` which means they don't hold any direct or indirect references to `allocator`.
        //   - This makes `ManuallyDrop::take(<state/worker_state>)` safe.
        //   - This makes `allocator_cell.into_inner()` safe.
        // - Of course, `Box::from_raw(allocator)` is safe because the allocator was properly leaked from a `Box<bumpalo::Bump>`.
        let (state, mut worker_state) = unsafe {
            let ret = (
                ManuallyDrop::take(state).into_inner(),
                ManuallyDrop::take(worker_state).into_inner(),
            );

            let allocator: Box<bumpalo::Bump> = Box::from_raw(allocator_cell.into_inner());
            drop(allocator);

            ret
        };

        for (track_id, note_descriptor) in deleted_notes {
            let track = worker_state.section_tracks.force_get_mut(track_id);
            match note_descriptor {
                crate::NoteDescriptor::Live { note_id, .. } => {
                    track
                        .live_notes
                        .remove(note_id)
                        .expect("tried to remove nonexistent note");
                }
                crate::NoteDescriptor::State { note_id, .. } => {
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
                tracing::warn!("error during thread join: {err:?}");
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
    pub index: u32,
    pub tx: crossbeam_channel::Sender<HostToWorkerEvent>,
    pub join_handle: thread::JoinHandle<()>,
}
impl WorkerHandle {
    pub fn new(
        index: u32,

        worker_tx: crossbeam_channel::Sender<WorkerToHostEvent>,
        worker_options: Arc<WorkerOptions>,

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
                    worker::run_forever(worker_tx, worker_rx, work_tx, work_rx, &worker_options)
                })
                .expect("failed to spawn thread"),
        }
    }
}

type WorkerJobSyncBuffer = SyncBuffer<&'static mut cubedaw_lib::Buffer, WorkerJob>;

#[must_use = "you should do something with the master output returned from this function"]
fn add_jobs(
    allocator: &'static bumpalo::Bump,
    work_tx: &crossbeam_channel::Sender<WorkerJob>,
    state: &'static State,
    worker_state: &'static mut WorkerHostState,
    worker_options: &WorkerOptions,
    start_pos_ref: Option<&mut PreciseSongPos>,
    _live_pos: PreciseSongPos, // TODO
) -> crate::sync::SyncAccessibleReadHandle<'static, &'static mut cubedaw_lib::Buffer, WorkerJob> {
    let allocate_sync_buffer = |alloc: &'static bumpalo::Bump| -> &'static WorkerJobSyncBuffer {
        let slice = alloc.alloc_slice_fill_copy(
            worker_options.buffer_size as usize / InternalBufferType::N,
            InternalBufferType::ZERO,
        );
        alloc.alloc(SyncBuffer::new(cubedaw_lib::Buffer::new_mut(slice)))
    };

    let master_output = allocate_sync_buffer(allocator);

    struct TrackTempData {
        sync_buffer: &'static WorkerJobSyncBuffer,
        job: WorkerJob,
    }
    let mut track_temp_map: IdMap<cubedaw_lib::Track, TrackTempData> = IdMap::new();

    // required due to borrowing rules
    let mut section_track_id_to_mutable_reference_to_section_track_data: IdMap<_, &'static mut _> =
        IdMap::new();
    let mut group_track_id_to_mutable_reference_to_group_track_data: IdMap<_, &'static mut _> =
        IdMap::new();

    let song_range_that_we_will_process = start_pos_ref.map(|start_pos_ref| {
        let start_pos = *start_pos_ref;
        let end_pos = start_pos
            + PreciseSongPos::from_song_pos_f32({
                // samples / (samples/second) / (60 seconds/minute) * beats/minute * units/beat
                worker_options.buffer_size as f32 / worker_options.sample_rate as f32 / 60.0
                    * state.bpm
                    * Range::UNITS_PER_BEAT as f32
            });
        // each consecutive range of start_pos to end_pos must result in consecutive song ranges
        // so don't use end_pos.ceil_to_song_pos() or whatever since that could result in overlap
        // which is very very bad and will cause very very bad things
        let song_range_that_we_will_process = Range::new(start_pos.song_pos, end_pos.song_pos);

        *start_pos_ref = end_pos;

        song_range_that_we_will_process
    });

    for (&track_id, section_track_data) in &mut worker_state.section_tracks {
        section_track_id_to_mutable_reference_to_section_track_data
            .insert(track_id, section_track_data);
    }
    for (&track_id, group_track_data) in &mut worker_state.group_tracks {
        group_track_id_to_mutable_reference_to_group_track_data.insert(track_id, group_track_data);
    }

    let mut track_stack = Vec::new();
    if state.tracks.has(state.root_track) {
        track_stack.push((state.root_track, master_output));
    }
    while let Some((track_id, group_input)) = track_stack.pop() {
        let sync_buffer = allocate_sync_buffer(allocator);

        let track = state.tracks.force_get(track_id);
        match track.inner {
            cubedaw_lib::TrackInner::Group(ref track_data) => {
                let worker_track_data = group_track_id_to_mutable_reference_to_group_track_data
                    .remove(track_id)
                    .unwrap();

                let job = WorkerJob::TrackGroup {
                    track_id,
                    nodes: &mut worker_track_data.nodes,
                    input: sync_buffer.get_read_handle(),
                    output: group_input.get_write_handle(),
                };

                track_temp_map.insert(track_id, TrackTempData { sync_buffer, job });

                for &track_id in &track_data.children {
                    track_stack.push((track_id, sync_buffer));
                }
            }
            cubedaw_lib::TrackInner::Section(ref track_data) => {
                let worker_track_data = section_track_id_to_mutable_reference_to_section_track_data
                    .remove(track_id)
                    .unwrap();

                let job = WorkerJob::TrackProcess {
                    track_id,
                    nodes: &mut worker_track_data.track_nodes,
                    input: sync_buffer.get_read_handle(),
                    output: group_input.get_write_handle(),
                };

                // live notes
                for (&live_note_id, note_state) in &mut worker_track_data.live_notes {
                    work_tx
                        .send(WorkerJob::NoteProcess {
                            track_id,
                            note_descriptor: crate::NoteDescriptor::Live {
                                note_id: live_note_id,
                                note: &note_state.note,
                                start_pos: note_state.start_pos,
                                samples_elapsed: note_state.samples_elapsed,
                            },
                            nodes: &mut note_state.nodes,
                            output: sync_buffer.get_write_handle(),
                        })
                        .unwrap();
                }

                // non-live notes
                {
                    // add the notes that started in this range to the worker state...
                    if let Some(song_range_that_we_will_process) = song_range_that_we_will_process {
                        for (section_range, section_id) in
                            track_data.sections_intersecting(song_range_that_we_will_process)
                        {
                            let section = track_data.section(section_id).unwrap();
                            for (_start_pos, note_id, _note) in section.note_start_positions_in(
                                section_range.intersect(song_range_that_we_will_process)
                                    - section_range.start,
                            ) {
                                worker_track_data.notes.insert(note_id, WorkerNoteState {
                                    section_id,
                                    nodes: worker_track_data.note_nodes.clone(),
                                });
                            }
                        }
                    }

                    // ...then process all notes
                    for (&note_id, note_state) in &mut worker_track_data.notes {
                        let (start_pos, note) = track_data
                            .section(note_state.section_id)
                            .unwrap()
                            .note(note_id)
                            .unwrap();
                        work_tx
                            .send(WorkerJob::NoteProcess {
                                track_id,
                                note_descriptor: crate::NoteDescriptor::State {
                                    note_id,
                                    start_pos,
                                    note,
                                },
                                nodes: &mut note_state.nodes,
                                output: sync_buffer.get_write_handle(),
                            })
                            .unwrap();
                    }
                }

                track_temp_map.insert(track_id, TrackTempData { sync_buffer, job })
            }
        }
    }
    debug_assert!(section_track_id_to_mutable_reference_to_section_track_data.is_empty());
    debug_assert!(group_track_id_to_mutable_reference_to_group_track_data.is_empty());

    // prime the SyncBuffers
    for (_, TrackTempData { sync_buffer, job }) in track_temp_map {
        if let Some(job) = sync_buffer.prime(job) {
            // sync_buffer.prime() returns Some(extra) when there are no writers
            // so just add the job to the queue
            work_tx.send(job).unwrap();
        }
    }

    let master_read_handle = master_output.get_read_handle();

    master_output.prime(WorkerJob::Finalize);

    master_read_handle
}
