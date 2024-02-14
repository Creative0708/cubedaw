use egui::ahash::HashMapExt;
use serde::{Deserialize, Serialize};

use crate::{
    track::{Note, Section, Track},
    Id, IdMap, Range,
};

#[derive(Default, Clone)]
pub struct State {
    pub song_range: Range,
    pub tracks: Vec<Id<Track>>,

    pub track_map: IdMap<Track, Track>,
    pub section_map: IdMap<Section, Section>,
    pub note_map: IdMap<Note, Note>,
}

impl State {
    pub fn new() -> Self {
        let mut tracks = Vec::new();
        let mut track_map = IdMap::new();
        for i in 1..=30 {
            let track = Track::dbg_new(format!("Track {}", i));
            tracks.push(track.id);
            track_map.insert(track.id, track);
        }
        Self {
            tracks,
            song_range: Range::from_beats(0, 64),

            track_map,
            section_map: IdMap::new(),
            note_map: IdMap::new(),
        }
    }
}

/// State changes used to efficiently broadcast changes to workers and also for the undo system
#[derive(Serialize, Deserialize)]
pub enum StateChange {
    DUMMY,
}

impl StateChange {
    pub fn apply(&self, state: &mut State) {
        todo!()
    }
    pub fn revert(&self, state: &mut State) {
        todo!()
    }
}
