use crate::util::{DragHandler, NodeSearch, SelectionRect};

pub struct EphemeralState {
    pub section_drag: DragHandler,
    pub note_drag: DragHandler,
    pub node_drag: DragHandler,

    pub selection_rect: SelectionRect,

    pub is_playing: bool,

    pub node_search: NodeSearch,
}

impl Default for EphemeralState {
    fn default() -> Self {
        Self {
            section_drag: DragHandler::new(),
            note_drag: DragHandler::new(),
            node_drag: DragHandler::new(),

            selection_rect: SelectionRect::new(),

            is_playing: false,

            node_search: NodeSearch::default(),
        }
    }
}
