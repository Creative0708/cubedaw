use cubedaw_lib::{Id, IdMap, NodeEntry, Note, Section, Track};
use egui::Pos2;

#[derive(Debug)]
pub struct UiState {
    pub tracks: IdMap<Track, TrackUiState>,
    pub show_root_track: bool,

    pub playhead_pos: i64,

    _private: private::Private,
}

mod private {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Private;
}

impl UiState {
    pub fn get_single_selected_track(&self) -> Option<Id<cubedaw_lib::Track>> {
        let mut single_selected_track = None;
        for (&track_id, track_ui_state) in &self.tracks {
            if track_ui_state.selected {
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
            show_root_track: false,

            playhead_pos: 0,

            _private: private::Private,
        }
    }
}

#[derive(Debug)]
pub struct TrackUiState {
    pub name: String,
    pub selected: bool,
    pub patch: PatchUiState,
    pub sections: IdMap<Section, SectionUiState>,
    /// Ordered track lists. This is the order of which the children of this track are displayed in the track tab.
    /// Unused when the track is a section track.
    pub track_list: Vec<Id<Track>>,
}

impl Default for TrackUiState {
    fn default() -> Self {
        Self {
            name: "Unnamed Track".into(),
            selected: false,
            patch: Default::default(),
            sections: Default::default(),
            track_list: Default::default(),
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
