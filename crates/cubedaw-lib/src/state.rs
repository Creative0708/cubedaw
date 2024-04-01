use crate::{id::IdMap, track::Track, Note, Range, Section};

#[derive(Debug)]
pub struct State {
    pub tracks: IdMap<Track>,
    pub sections: IdMap<Section>,
    pub notes: IdMap<Note>,
    pub song_boundary: Range,
}

impl State {
    pub fn tracking() -> Self {
        Self {
            sections: IdMap::tracking(),
            tracks: IdMap::tracking(),
            notes: IdMap::tracking(),
            song_boundary: Range::new(0, 16 * Range::UNITS_PER_BEAT * 4),
        }
    }
}
