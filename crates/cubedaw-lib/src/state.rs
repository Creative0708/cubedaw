use crate::{id::IdMap, track::Track, Note, Range, Section};

#[derive(Debug)]
pub struct State {
    // TODO bpm can vary over time, implement that
    pub bpm: f32,

    pub tracks: IdMap<Track>,
    pub sections: IdMap<Section>,
    pub notes: IdMap<Note>,
    pub song_boundary: Range,
}

impl State {
    pub fn new() -> Self {
        Self {
            bpm: 120.0,

            tracks: IdMap::new(),
            sections: IdMap::new(),
            notes: IdMap::new(),
            song_boundary: Range::new(0, 16 * Range::UNITS_PER_BEAT * 4),
        }
    }

    // pub fn clear_events(&mut self) {
    //     self.tracks.clear_events();
    //     self.sections.clear_events();
    //     self.notes.clear_events();
    // }
}
