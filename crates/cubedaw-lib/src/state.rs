use crate::{id::IdMap, track::Track, Note, Range, Section};

#[derive(Debug)]
pub struct State {
    // TODO bpm can vary over time, implement that
    pub bpm: f32,

    pub tracks: IdMap<Track>,
    pub sections: IdMap<Section>,
    pub notes: IdMap<Note>,
    pub song_boundary: Range,

    // TODO is needle the right term for this? it's like the "play cursor" that plays the notes and such
    // also is this precise enough?
    pub needle_pos: f32,
}

impl State {
    pub fn tracking() -> Self {
        Self {
            bpm: 120.0,

            sections: IdMap::tracking(),
            tracks: IdMap::tracking(),
            notes: IdMap::tracking(),
            song_boundary: Range::new(0, 16 * Range::UNITS_PER_BEAT * 4),

            needle_pos: 0.0,
        }
    }

    pub fn clear_events(&mut self) {
        self.tracks.clear_events();
        self.sections.clear_events();
        self.notes.clear_events();
    }
}
