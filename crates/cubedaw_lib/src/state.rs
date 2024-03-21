use crate::{id::IdMap, track::Track, Range, Section};

#[derive(Debug, Default)]
pub struct State {
    pub sections: IdMap<Section>,
    pub tracks: IdMap<Track>,
    pub song_boundary: Range,
}

impl State {}
