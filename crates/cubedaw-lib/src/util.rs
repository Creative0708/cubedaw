#[derive(Clone, Copy, Debug)]
/// A precise position in the song. Mainly used for rendering.
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
            fraction: ((song_pos - floor) * 18446744073709552000_f32) as u64,
        }
    }
    pub fn from_song_pos_f64(song_pos: f64) -> Self {
        let floor = song_pos.floor();
        Self {
            song_pos: floor as i64,
            fraction: ((song_pos - floor) * 18446744073709552000_f64) as u64,
        }
    }
    pub fn to_song_pos_f32(self) -> f32 {
        self.song_pos as f32 + self.fraction as f32 * 5.421011e-20_f32
    }
    pub fn to_song_pos_f64(self) -> f64 {
        self.song_pos as f64 + self.fraction as f64 * 5.421010862427522e-20_f64
    }

    pub fn ceil_to_song_pos(self) -> i64 {
        self.song_pos + if self.fraction > 0 { 1 } else { 0 }
    }
    pub fn round_to_song_pos(self) -> i64 {
        // 9223372036854775808 == 2 ** 63
        self.song_pos
            + if self.fraction >= 9223372036854775808 {
                1
            } else {
                0
            }
    }
}
impl std::ops::Add<Self> for PreciseSongPos {
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
impl std::ops::Sub<Self> for PreciseSongPos {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let (fraction, carry) = self.fraction.overflowing_sub(rhs.fraction);
        let (song_pos, carry) = {
            // change to carrying_sub when https://github.com/rust-lang/rust/issues/85532 is stabilized
            let (a, b) = self.song_pos.overflowing_sub(rhs.song_pos);
            let (c, d) = a.overflowing_sub(carry as _);
            (c, b != d)
        };
        debug_assert!(!carry, "SamplePos::sub overflowed");
        Self { song_pos, fraction }
    }
}
impl Default for PreciseSongPos {
    fn default() -> Self {
        Self::ZERO
    }
}
