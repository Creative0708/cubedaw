use std::{collections::VecDeque, sync::mpsc, thread};

use cubedaw_command::StateCommandWrapper;
use cubedaw_workerlib::{SamplePos, WorkerOptions};

pub struct WorkerHostHandle {
    tx: mpsc::Sender<WorkerHostHandleEvent>,
    join_handle: thread::JoinHandle<()>,
}

impl WorkerHostHandle {
    pub fn new(state: cubedaw_lib::State, options: WorkerOptions) -> Self {
        let (tx, rx) = mpsc::channel();
        WorkerHostHandle {
            tx,
            join_handle: thread::Builder::new()
                .name("Audio Worker Host".into())
                .spawn(move || worker_host(rx, state, options))
                .expect("failed to spawn thread"),
        }
    }

    pub fn init_workers(&mut self, num_workers: usize) {
        self.tx
            .send(WorkerHostHandleEvent::InitWorkers { num_workers })
            .expect("channel closed???");
    }
    pub fn start_processing(&mut self, from: i64) {
        self.tx
            .send(WorkerHostHandleEvent::StartProcessing { from })
            .expect("channel closed???");
    }
}

enum WorkerHostHandleEvent {
    InitWorkers { num_workers: usize },
    Options(WorkerOptions),
    StartProcessing { from: i64 },
    StopProcessing,
    QueueCommands(Box<[Box<dyn StateCommandWrapper>]>),
}

fn worker_host(
    rx: mpsc::Receiver<WorkerHostHandleEvent>,
    state: cubedaw_lib::State,
    mut options: WorkerOptions,
) {
    let mut host = cubedaw_worker::WorkerHost::new(state, options.clone());

    let mut processing = false;

    let mut queued_commands = VecDeque::new();

    let mut playhead_pos = SamplePos::new(i64::MIN, 0.0);

    'out: loop {
        while let Some(event) = if processing {
            match rx.try_recv() {
                Ok(event) => Some(event),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => break 'out,
            }
        } else {
            match rx.recv() {
                Ok(event) => Some(event),
                Err(mpsc::RecvError) => break 'out,
            }
        } {
            match event {
                WorkerHostHandleEvent::InitWorkers { num_workers } => {
                    host.init_workers(num_workers);
                }
                WorkerHostHandleEvent::StartProcessing { from } => {
                    processing = true;
                    playhead_pos = SamplePos::from_song_pos(from);
                }
                WorkerHostHandleEvent::StopProcessing => {
                    processing = false;
                }
                WorkerHostHandleEvent::QueueCommands(commands) => {
                    for event in commands.into_vec() {
                        queued_commands.push_back(event);
                    }
                }
                WorkerHostHandleEvent::Options(new_options) => {
                    options.clone_from(&new_options);
                    host.set_options(new_options);
                }
            }
        }
        if processing {
            let state = host.collect();
            for command in queued_commands.iter_mut() {
                command.execute(state);
            }
            host.queue(playhead_pos);
        }
    }

    host.join();
}
