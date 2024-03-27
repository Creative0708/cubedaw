use crate::{id::IdMap, track::Track, Range, Section};

#[derive(Debug)]
pub struct State {
    pub sections: IdMap<Section>,
    pub tracks: IdMap<Track>,
    pub song_boundary: Range,
}

impl State {
    pub fn tracking() -> Self {
        Self {
            sections: IdMap::tracking(),
            tracks: IdMap::tracking(),
            song_boundary: Range::new(0, 16 * Range::UNITS_PER_BEAT * 4),
        }
    }
}
