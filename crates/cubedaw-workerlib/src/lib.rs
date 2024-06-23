mod registry;
use std::sync::Arc;
mod sync;
pub(crate) use sync::{SyncCumulativeBuffer, SyncCumulativeBufferGuard};

pub use registry::{DynNodeFactory, NodeRegistry};
mod state;
pub use state::{WorkerJob, WorkerSectionTrackState, WorkerState};
mod buffer;
pub use buffer::Buffer;
pub mod nodes;

#[derive(Clone)]
pub struct WorkerOptions {
    pub node_registry: Arc<crate::NodeRegistry>,

    pub sample_rate: u32,
    pub buffer_size: u32,
}

#[derive(Clone, Copy)]
pub struct SamplePos {
    pub song_pos: i64,
    pub sample: f32,
}

impl SamplePos {
    pub fn new(song_pos: i64, sample: f32) -> Self {
        Self { song_pos, sample }
    }
    pub fn from_song_pos(song_pos: i64) -> Self {
        Self {
            song_pos,
            sample: 0.0,
        }
    }

    // samples_per_unit = samples_per_second / beats_per_second / units_per_beat
    // = sample_rate / (bpm / 60) / Range::UNITS_PER_BEAT
    pub fn add(self, rhs: Self, samples_per_unit: f32) -> Self {
        let mut song_pos = self.song_pos + rhs.song_pos;
        let mut sample = self.sample + rhs.sample;
        if sample >= samples_per_unit {
            sample -= samples_per_unit;
            song_pos += 1;
        }
        Self { song_pos, sample }
    }
}
