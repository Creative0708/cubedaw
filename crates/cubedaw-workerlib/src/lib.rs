use std::{ops, sync::Arc};
mod sync;
pub(crate) use sync::SyncBuffer;

pub use cubedaw_lib::{DynNodeFactory, NodeRegistry};
mod state;
pub use state::{WorkerJob, WorkerJobResult, WorkerSectionTrackState, WorkerState};

mod node_graph;

#[derive(Clone, Default, Debug)]
/// Static worker options. These don't change (unless the worker host is reloaded.)
pub struct WorkerOptions {
    pub registry: Arc<crate::NodeRegistry>,

    pub num_workers: u32,

    pub sample_rate: u32,
    pub buffer_size: u32,
}

#[derive(Clone, Copy, Debug)]
/// A precise position in the song, used for rendering.
pub struct PreciseSongPos {
    /// Song position, rounded down.
    pub song_pos: i64,
    /// Fractional song position. This is mapped from 0..=u64::MAX to a range from 0..1. (It's like fixed point!)
    pub fraction: u64,
}

impl PreciseSongPos {
    pub const ZERO: Self = Self {
        song_pos: 0,
        fraction: 0,
    };
    pub fn new(song_pos: i64, fraction: u64) -> Self {
        Self { song_pos, fraction }
    }
    pub fn from_song_pos(song_pos: i64) -> Self {
        Self {
            song_pos,
            fraction: 0,
        }
    }
    pub fn from_song_pos_f32(song_pos: f32) -> Self {
        let floor = song_pos.floor();
        Self {
            song_pos: floor as i64,
            // 18446744073709551616 == 2 ** 64
            fraction: ((song_pos - floor) * 18446744073709551616_f32) as u64,
        }
    }

    pub fn ceil_to_song_pos(self) -> i64 {
        self.song_pos + if self.fraction > 0 { 1 } else { 0 }
    }
}
impl ops::Add<Self> for PreciseSongPos {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let (fraction, carry) = self.fraction.overflowing_add(rhs.fraction);
        let (song_pos, carry) = {
            // change to carrying_add when https://github.com/rust-lang/rust/issues/85532 is stabilized
            let (a, b) = self.song_pos.overflowing_add(rhs.song_pos);
            let (c, d) = a.overflowing_add(carry as _);
            (c, b != d)
        };
        debug_assert!(!carry, "SamplePos::add overflowed");
        Self { song_pos, fraction }
    }
}
impl Default for PreciseSongPos {
    fn default() -> Self {
        Self::ZERO
    }
}
