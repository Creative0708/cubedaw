use crate::{id::IdMap, track::Track, Id, Range};

#[derive(Debug, Clone)]
pub struct State {
    // TODO implement bpm automation (after non-bpm automation is done ofc)
    pub bpm: f32,

    pub tracks: IdMap<Track>,
    pub root_track: Id<Track>,
    pub song_boundary: Range,
}

const _: () = {
    // Check for surface-level interior mutability
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<State>()
};

impl State {
    pub fn add_time_to_position(
        &self,
        pos: crate::PreciseSongPos,
        duration: std::time::Duration,
    ) -> crate::PreciseSongPos {
        // when bpm automation exists this will have to be changed
        let units = duration.as_secs_f64() / 60.0 * self.bpm as f64 * Range::UNITS_PER_BEAT as f64;
        pos + crate::PreciseSongPos::from_song_pos_f64(units)
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            bpm: 120.0,

            tracks: IdMap::new(),
            root_track: Id::invalid(),
            song_boundary: Range::new(0, 16 * Range::UNITS_PER_BEAT as i64 * 4),
        }
    }
}
