use crate::util::{DragHandler, SelectionRect};

pub struct EphemeralState {
    pub section_drag: DragHandler,
    pub note_drag: DragHandler,

    pub selection_rect: SelectionRect,

    pub is_playing: bool,
}

impl EphemeralState {
    pub fn new() -> Self {
        Self {
            section_drag: DragHandler::new(),
            note_drag: DragHandler::new(),

            selection_rect: SelectionRect::new(),

            is_playing: false,
        }
    }
}
