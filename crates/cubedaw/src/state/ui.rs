use cubedaw_lib::{Id, IdMap, NodeData, NodeEntry, Note, PreciseSongPos, Section, Track};
use egui::Pos2;

#[derive(Debug)]
pub struct UiState {
    pub tracks: IdMap<Track, TrackUiState>,

    // An ordered track list. This is the order with which the tracks are displayed in the track tab.
    pub track_list: Vec<Id<Track>>,

    pub playhead_pos: i64,
}

impl UiState {
    pub fn get_single_selected_track(&self) -> Option<Id<cubedaw_lib::Track>> {
        let mut single_selected_track = None;
        for &track_id in &self.track_list {
            let track = self
                .tracks
                .get(track_id)
                .expect("ui_state.track_list not synchronized with ui_state.tracks");
            if track.selected {
                if single_selected_track.is_some() {
                    // more than one selected track, give up
                    single_selected_track = None;
                    break;
                } else {
                    single_selected_track = Some(track_id);
                }
            }
        }
        single_selected_track
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            tracks: IdMap::new(),

            track_list: Vec::new(),

            playhead_pos: 0,
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
    pub nodes: IdMap<NodeEntry, NodeUiState>,
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
    pub pos: Pos2,
    pub width: f32,
}
