use cubedaw_lib::{IdMap, Section, State, Track};

#[derive(Debug)]
pub struct UiState {
    pub sections: IdMap<Section, SectionUiState>,
    pub tracks: IdMap<Track, TrackUiState>,
}

impl UiState {
    pub fn track(&mut self, state: &State) {
        self.sections.track(&state.sections);
        self.tracks.track(&state.tracks);
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sections: IdMap::nontracking(),
            tracks: IdMap::nontracking(),
        }
    }
}

#[derive(Debug, Default)]
pub struct SectionUiState {
    pub selected: bool,
}
#[derive(Debug, Default)]
pub struct TrackUiState {
    pub selected: bool,
}
