use cubedaw_lib::{Id, IdMap, NodeData, Note, Section, Track};
use egui::Vec2;

#[derive(Debug)]
pub struct UiState {
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
            tracks: IdMap::new(),

            track_list: Vec::new(),

            playhead_pos: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct TrackUiState {
    pub name: String,
    pub selected: bool,
    pub patch: PatchUiState,
    pub sections: IdMap<Section, SectionUiState>,
}

impl Default for TrackUiState {
    fn default() -> Self {
        Self {
            name: "Unnamed Track".into(),
            selected: false,
            patch: PatchUiState::default(),
            sections: IdMap::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct PatchUiState {
    pub nodes: IdMap<NodeData, NodeUiState>,
}

#[derive(Debug)]
pub struct SectionUiState {
    pub name: String,
    pub selected: bool,
    pub notes: IdMap<Note, NoteUiState>,
}

impl Default for SectionUiState {
    fn default() -> Self {
        Self {
            name: "Unnamed Section".into(),
            selected: false,
            notes: IdMap::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct NoteUiState {
    pub selected: bool,
}

#[derive(Debug)]
pub struct NodeUiState {
    pub selected: bool,
    pub pos: Vec2,
    pub width: f32,
}
