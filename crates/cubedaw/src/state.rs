use cubedaw_lib::{IdMap, Note, Section, State, Track};

use crate::util::{DragHandler, SelectableUiData};

#[derive(Debug)]
pub struct UiState {
    pub sections: IdMap<Section, SectionUiState>,
    pub notes: IdMap<Note, NoteUiState>,
    pub tracks: IdMap<Track, TrackUiState>,

    pub section_drag: DragHandler,
    pub note_drag: DragHandler,
}

impl UiState {
    pub fn track(&mut self, state: &State) {
        self.sections.track(&state.sections);
        self.tracks.track(&state.tracks);
        self.notes.track(&state.notes);
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sections: IdMap::nontracking(),
            tracks: IdMap::nontracking(),
            notes: IdMap::nontracking(),

            section_drag: DragHandler::new(),
            note_drag: DragHandler::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct SectionUiState {
    pub selected: bool,
}

impl SelectableUiData<Section> for SectionUiState {
    fn selected(&self) -> bool {
        self.selected
    }
    fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
}

#[derive(Debug, Default)]
pub struct TrackUiState {
    pub selected: bool,
}

impl SelectableUiData<Track> for TrackUiState {
    fn selected(&self) -> bool {
        self.selected
    }
    fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
}

#[derive(Debug, Default)]
pub struct NoteUiState {
    pub selected: bool,
}

impl SelectableUiData<Note> for NoteUiState {
    fn selected(&self) -> bool {
        self.selected
    }
    fn set_selected(&mut self, selected: bool) {
        self.selected = selected;
    }
}
