use std::{collections::VecDeque, io::Write};

use cpal::traits::DeviceTrait;
use cubedaw_lib::InternalBufferType;
use cubedaw_worker::WorkerOptions;

pub struct CpalAudioHandler {
    pub inner: CpalAudioHandlerState,
}

impl CpalAudioHandler {
    /// Creates a new, empty `CpalAudioHandler` that's not connected to anything.
    pub fn new() -> Self {
        Self {
            inner: CpalAudioHandlerState::Offline,
        }
    }

    pub fn close(&mut self) {
        replace_with::replace_with_or_default(&mut self.inner, |inner| match inner {
            CpalAudioHandlerState::Open {
                audio_device,
                audio_stream: _,
                output_ring_buffer: _,
            } => CpalAudioHandlerState::Closed { audio_device },
            other => other,
        });
    }
    pub fn make_offline(&mut self) {
        self.inner = CpalAudioHandlerState::Offline;
    }

    pub fn is_open(&self) -> bool {
        matches!(self.inner, CpalAudioHandlerState::Open { .. })
    }
    pub fn open(&mut self, options: &WorkerOptions) {
        replace_with::replace_with_or_default(&mut self.inner, |inner| match inner {
            CpalAudioHandlerState::Offline => panic!("can't open an audio handler with no device!"),
            CpalAudioHandlerState::Closed { audio_device } => {
                let (tx, rx) = crossbeam_channel::bounded::<InternalBufferType>(
                    options.buffer_size as usize / InternalBufferType::N * 16,
                ); // TODO make configurable

                let mut ring_buffer: VecDeque<f32> =
                    VecDeque::with_capacity(options.buffer_size as usize * 2);
                // ring_buffer.extend(std::iter::repeat_n(0.0, options.buffer_size as usize));
                CpalAudioHandlerState::Open {
                    audio_stream: audio_device
                        .build_output_stream(
                            &cpal::StreamConfig {
                                channels: 1,
                                sample_rate: cpal::SampleRate(options.sample_rate),
                                buffer_size: cpal::BufferSize::Fixed(options.buffer_size),
                            },
                            move |buffer: &mut [f32], _info| {
                                for val in buffer.iter_mut() {
                                    if ring_buffer.is_empty() {
                                        ring_buffer.extend(
                                            rx.try_recv()
                                                .unwrap_or_else(|_| {
                                                    // TODO: keep track of buffer underflows
                                                    // eprintln!("buffer underflow :(");
                                                    bytemuck::zeroed()
                                                })
                                                .as_array(),
                                        );
                                    }
                                    let unclamped_val =
                                        ring_buffer.pop_front().expect("unreachable");
                                    *val = unclamped_val.clamp(-1.0, 1.0);
                                }

                                // for debugging purposes
                                let max = buffer.iter().copied().map(f32::abs).fold(0.0, f32::max);
                                use std::time::SystemTime;
                                let millis = SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap()
                                    .subsec_millis();
                                let mut stdout = std::io::stdout();
                                write!(&mut stdout, "\x1b[0K {millis:03}ms: max {max}\r").unwrap();
                                stdout.flush().unwrap();
                            },
                            |err| todo!("{err:?}"),
                            None,
                        )
                        .expect("failed to build output stream"),
                    output_ring_buffer: tx,

                    audio_device,
                }
            }
            already_open @ CpalAudioHandlerState::Open { .. } => already_open,
        });
    }

    pub fn send(&mut self, data: InternalBufferType) {
        if let CpalAudioHandlerState::Open {
            audio_device: _,
            audio_stream: _,
            ref mut output_ring_buffer,
        } = self.inner
        {
            match output_ring_buffer.try_send(data) {
                Ok(()) => (),
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    panic!("channel disconnected");
                }
                Err(crossbeam_channel::TrySendError::Full(_)) => {
                    eprintln!("buffer overflow :( (not in the memory safety way)");
                }
            }
        }
    }

    pub fn set_device(&mut self, device: cpal::Device, options: &WorkerOptions) {
        let was_open = self.is_open();
        self.inner = CpalAudioHandlerState::Closed {
            audio_device: device,
        };
        if was_open {
            self.open(options);
        }
    }
}

#[derive(Default)]
pub enum CpalAudioHandlerState {
    #[default]
    Offline,
    Closed {
        audio_device: cpal::Device,
    },
    Open {
        audio_device: cpal::Device,
        audio_stream: cpal::Stream,

        output_ring_buffer: crossbeam_channel::Sender<InternalBufferType>,
    },
}
