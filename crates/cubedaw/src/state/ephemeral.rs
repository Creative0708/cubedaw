use cubedaw_lib::{IdMap, NodeData, NodeEntry, Track};
use egui::Vec2;

use crate::util::{DragHandler, NodeSearch, SelectionRect};

#[derive(Debug)]
pub struct EphemeralState {
    pub section_drag: DragHandler,
    pub note_drag: DragHandler,
    pub tracks: IdMap<Track, TrackEphemeralState>,

    pub selection_rect: SelectionRect,

    pub node_search: NodeSearch,
}

impl Default for EphemeralState {
    fn default() -> Self {
        Self {
            section_drag: DragHandler::new(),
            note_drag: DragHandler::new(),
            tracks: IdMap::new(),

            selection_rect: SelectionRect::new(),

            node_search: NodeSearch::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct TrackEphemeralState {
    pub node_drag: DragHandler,
    pub nodes: IdMap<NodeEntry, NodeEphemeralState>,
}

#[derive(Debug, Default)]
pub struct NodeEphemeralState {
    pub size: Vec2,
}
