use crate::{
    id::IdMap,
    track::{GroupTrack, Track},
    Range,
};

#[derive(Debug, Clone)]
pub struct State {
    // TODO bpm can vary over time, implement that
    pub bpm: f32,

    pub tracks: IdMap<Track>,
    pub root_track: GroupTrack,
    pub song_boundary: Range,
}

const _: () = {
    // Check for surface-level interior mutability
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<State>()
};

impl State {
    // pub fn clear_events(&mut self) {
    //     self.tracks.clear_events();
    //     self.sections.clear_events();
    //     self.notes.clear_events();
    // }
}

impl Default for State {
    fn default() -> Self {
        Self {
            bpm: 120.0,

            tracks: IdMap::new(),
            root_track: GroupTrack::new(),
            song_boundary: Range::new(0, 16 * Range::UNITS_PER_BEAT * 4),
        }
    }
}
