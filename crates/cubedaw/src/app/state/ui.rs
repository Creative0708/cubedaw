use cubedaw_lib::{Clip, Id, IdMap, Node, Note, Track};
use egui::Pos2;

use crate::util::Select;

#[derive(Debug)]
pub struct UiState {
    pub tracks: IdMap<Track, TrackUiState>,

    pub show_root_track: bool,

    pub playhead_pos: i64,

    pub _private: private::Private,
}

mod private {
    #[derive(Clone, Copy, Debug)]
    pub struct Private;
}

impl UiState {
    pub(in crate::app) fn new() -> Self {
        Self {
            tracks: IdMap::new(),
            show_root_track: false,

            playhead_pos: 0,

            _private: private::Private,
        }
    }
    pub fn get_single_selected_track(&self) -> Option<Id<cubedaw_lib::Track>> {
        let mut single_selected_track = None;
        for (track_id, track_ui_state) in &self.tracks {
            if track_ui_state.select.is() {
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

#[derive(Debug)]
pub struct TrackUiState {
    pub name: String,
    pub select: Select,
    pub patch: PatchUiState,
    pub clips: IdMap<Clip, ClipUiState>,

    /// Whether the track has its children hidden or not.
    pub closed: bool,

    /// Ordered track lists. This is the order of which the children of this track are displayed in the track tab.
    pub track_list: Vec<Id<Track>>,
}

impl Default for TrackUiState {
    fn default() -> Self {
        Self {
            name: "Unnamed Track".into(),
            select: Default::default(),
            patch: Default::default(),
            clips: Default::default(),

            closed: false,

            track_list: Default::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct PatchUiState {
    pub nodes: IdMap<Node, NodeUiState>,
}

#[derive(Debug)]
pub struct ClipUiState {
    pub name: String,
    pub select: Select,
    pub notes: IdMap<Note, NoteUiState>,
}

impl Default for ClipUiState {
    fn default() -> Self {
        Self {
            name: "Unnamed Clip".into(),
            select: Select::Deselect,
            notes: IdMap::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct NoteUiState {
    pub select: Select,
}

#[derive(Debug)]
pub struct NodeUiState {
    pub select: Select,
    pub pos: Pos2,
    pub width: f32,
}
