use std::{collections::VecDeque, sync::mpsc, thread};

use cubedaw_command::StateCommandWrapper;
use cubedaw_workerlib::{PreciseSongPos, WorkerOptions};

pub struct WorkerHostHandle {
    tx: mpsc::Sender<AppToWorkerHostEvent>,
    rx: mpsc::Receiver<WorkerHostToAppEvent>,
    join_handle: thread::JoinHandle<()>,

    is_playing: bool,
    last_playhead_update: Option<(PreciseSongPos, std::time::Instant)>,
}

impl WorkerHostHandle {
    pub fn new() -> Self {
        let (app_tx, worker_rx) = mpsc::channel();
        let (worker_tx, app_rx) = mpsc::channel();
        WorkerHostHandle {
            tx: app_tx,
            rx: app_rx,
            join_handle: thread::Builder::new()
                .name("Audio Worker Host".into())
                .spawn(move || worker_host(worker_rx, worker_tx))
                .expect("failed to spawn thread"),

            is_playing: false,
            last_playhead_update: None,
        }
    }

    pub fn init(
        &mut self,
        num_workers: usize,
        state: cubedaw_lib::State,
        worker_options: WorkerOptions,
    ) {
        self.tx
            .send(AppToWorkerHostEvent::Init {
                num_workers,
                state,
                options: worker_options,
            })
            .expect("channel closed???");
    }
    pub fn start_processing(&mut self, from: i64) {
        self.tx
            .send(AppToWorkerHostEvent::StartProcessing { from })
            .expect("channel closed???");
        self.is_playing = true;
    }
    pub fn stop_processing(&mut self) {
        self.tx
            .send(AppToWorkerHostEvent::StopProcessing)
            .expect("channel closed???");
        self.is_playing = false;
        self.last_playhead_update = None;
    }
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn try_recv(&self) -> Option<WorkerHostToAppEvent> {
        match self.rx.try_recv() {
            Ok(event) => Some(event),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => panic!("channel closed???"),
        }
    }
    fn recv(&self) -> WorkerHostToAppEvent {
        self.rx.recv().expect("channel closed???")
    }

    pub fn last_playhead_update(&self) -> Option<(PreciseSongPos, std::time::Instant)> {
        self.last_playhead_update
    }
}

#[derive(Debug)]
enum AppToWorkerHostEvent {
    Init {
        num_workers: usize,
        state: cubedaw_lib::State,
        options: WorkerOptions,
    },
    StartProcessing {
        from: i64,
    },
    StopProcessing,
    Commands(Box<[Box<dyn StateCommandWrapper>]>),
}

#[derive(Debug)]
enum WorkerHostToAppEvent {
    PlayheadUpdate {
        pos: PreciseSongPos,
        timestamp: std::time::Instant,
    },
}

fn worker_host(rx: mpsc::Receiver<AppToWorkerHostEvent>, tx: mpsc::Sender<WorkerHostToAppEvent>) {
    use cubedaw_worker::WorkerHost;

    let Ok(first_event) = rx.recv() else { return };
    let AppToWorkerHostEvent::Init {
        num_workers,
        state,
        options,
    } = first_event
    else {
        panic!("other event sent to worker_host before Init: {first_event:?}");
    };

    let mut idle_host = cubedaw_worker::WorkerHost::new(num_workers, state, options);
    let mut is_playing = false;

    let mut playhead_pos = Default::default();

    'outer: loop {
        // process events first
        loop {
            let event = if is_playing {
                match rx.try_recv() {
                    Ok(event) => event,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => break 'outer,
                }
            } else {
                match rx.recv() {
                    Ok(event) => event,
                    Err(mpsc::RecvError) => break 'outer,
                }
            };

            match event {
                AppToWorkerHostEvent::Init {
                    num_workers,
                    state,
                    options,
                } => {
                    idle_host.join();
                    idle_host = cubedaw_worker::WorkerHost::new(num_workers, state, options);
                }
                AppToWorkerHostEvent::StartProcessing { from } => {
                    playhead_pos = PreciseSongPos::from_song_pos(from);
                    is_playing = true;
                }
                AppToWorkerHostEvent::StopProcessing => {
                    is_playing = false;
                }
                AppToWorkerHostEvent::Commands(commands) => {
                    for mut command in commands.into_vec() {
                        command.execute(idle_host.state_mut());
                    }
                }
            }
        }
        // Process audio
        playhead_pos = idle_host.process(playhead_pos);
    }

    idle_host;
}
