use std::{sync::mpsc, thread};

use cubedaw_command::StateCommandWrapper;
use cubedaw_worker::WorkerOptions;

pub struct WorkerHostHandle {
    tx: mpsc::Sender<AppToWorkerHostEvent>,
    rx: mpsc::Receiver<WorkerHostToAppEvent>,
    join_handle: thread::JoinHandle<()>,

    is_playing: bool,
    is_init: bool,
    last_playhead_update: Option<(cubedaw_lib::PreciseSongPos, std::time::Instant)>,
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
            is_init: false,
            last_playhead_update: None,
        }
    }

    pub fn init(&mut self, state: cubedaw_lib::State, worker_options: WorkerOptions) {
        self.tx
            .send(AppToWorkerHostEvent::Init {
                state,
                options: worker_options,
            })
            .expect("channel closed???");

        self.is_init = true;
    }
    pub fn start_playing(&mut self, from: i64) {
        self.tx
            .send(AppToWorkerHostEvent::StartPlaying { from })
            .expect("channel closed???");
        self.is_playing = true;
        self.last_playhead_update = Some((
            cubedaw_lib::PreciseSongPos::from_song_pos(from),
            std::time::Instant::now(),
        ));
    }
    pub fn stop_processing(&mut self) {
        self.tx
            .send(AppToWorkerHostEvent::StopPlaying)
            .expect("channel closed???");
        self.is_playing = false;
        self.last_playhead_update = None;
    }
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }
    pub fn is_init(&self) -> bool {
        self.is_init
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

    pub fn handle_events(&mut self) {
        while let Some(event) = self.try_recv() {
            match event {
                WorkerHostToAppEvent::PlayheadUpdate { pos, timestamp } => {
                    dbg!((pos, timestamp));
                    self.last_playhead_update = Some((pos, timestamp));
                }
            }
        }
    }

    pub fn last_playhead_update(
        &self,
    ) -> Option<(cubedaw_lib::PreciseSongPos, std::time::Instant)> {
        self.last_playhead_update
    }
}

#[derive(Debug)]
enum AppToWorkerHostEvent {
    Init {
        state: cubedaw_lib::State,
        options: WorkerOptions,
    },
    StartPlaying {
        from: i64,
    },
    StopPlaying,
    UpdatePlayheadPos(i64),
    Commands(Box<[Box<dyn StateCommandWrapper>]>),
}

#[derive(Debug)]
enum WorkerHostToAppEvent {
    PlayheadUpdate {
        pos: cubedaw_lib::PreciseSongPos,
        timestamp: std::time::Instant,
    },
}

fn worker_host(rx: mpsc::Receiver<AppToWorkerHostEvent>, tx: mpsc::Sender<WorkerHostToAppEvent>) {
    use std::time::{Duration, Instant};

    let Ok(first_event) = rx.recv() else { return };
    let AppToWorkerHostEvent::Init { state, options } = first_event else {
        panic!("other event sent to worker_host before Init: {first_event:?}");
    };

    let mut time_to_wait_until = Instant::now();
    let mut duration_per_frame =
        Duration::from_secs_f64(options.buffer_size as f64 / options.sample_rate as f64);

    let mut idle_host = cubedaw_worker::WorkerHost::new(state, options);
    let mut is_playing = false;

    let mut playhead_pos = Default::default();

    'outer: loop {
        // process events first
        loop {
            let event = match rx.try_recv() {
                Ok(event) => event,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break 'outer,
            };

            match event {
                AppToWorkerHostEvent::Init { state, options } => {
                    idle_host.join();

                    time_to_wait_until = Instant::now();
                    duration_per_frame = Duration::from_secs_f64(
                        options.buffer_size as f64 / options.sample_rate as f64,
                    );

                    idle_host = cubedaw_worker::WorkerHost::new(state, options);
                }
                AppToWorkerHostEvent::StartPlaying { from } => {
                    playhead_pos = cubedaw_lib::PreciseSongPos::from_song_pos(from);
                    is_playing = true;
                }
                AppToWorkerHostEvent::StopPlaying => {
                    is_playing = false;
                }
                AppToWorkerHostEvent::UpdatePlayheadPos(pos) => {
                    playhead_pos = cubedaw_lib::PreciseSongPos::from_song_pos(pos);
                }
                AppToWorkerHostEvent::Commands(commands) => {
                    for mut command in commands.into_vec() {
                        command.execute(idle_host.state_mut());
                    }
                }
            }
        }
        let live_playhead_pos = playhead_pos;
        // Process audio
        idle_host = idle_host.process(
            if is_playing {
                Some(&mut playhead_pos)
            } else {
                None
            },
            live_playhead_pos,
        );
        time_to_wait_until += duration_per_frame;

        let now = Instant::now();
        // dbg!(now);
        if now < time_to_wait_until {
            std::thread::sleep(time_to_wait_until - now);
        } else {
            log::warn!(
                "audio workerhost underflow: behind by {:.02} ms",
                (now - time_to_wait_until).as_secs_f64() * 1000.0
            );
        }
    }
}
