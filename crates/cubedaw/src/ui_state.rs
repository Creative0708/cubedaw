use cubedaw_lib::{Id, IdMap, Note, Section, Track};

#[derive(Debug)]
pub struct UiState {
    pub sections: IdMap<Section, SectionUiState>,
    pub notes: IdMap<Note, NoteUiState>,
    pub tracks: IdMap<Track, TrackUiState>,

    // An ordered track list. This is the order with which the tracks are displayed in the track tab.
    pub track_list: Vec<Id<Track>>,

    // TODO is this precise enough?
    pub playhead_pos: f32,
}

impl UiState {}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sections: IdMap::new(),
            tracks: IdMap::new(),
            notes: IdMap::new(),

            track_list: Vec::new(),

            playhead_pos: 0.0,
        }
    }
}

#[derive(Debug, Default)]
pub struct SectionUiState {
    pub selected: bool,
}

#[derive(Debug)]
pub struct TrackUiState {
    pub name: String,
    pub selected: bool,
}

impl Default for TrackUiState {
    fn default() -> Self {
        Self {
            name: "Unnamed Track".into(),
            selected: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct NoteUiState {
    pub selected: bool,
}
