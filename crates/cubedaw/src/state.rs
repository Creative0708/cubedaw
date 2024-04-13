use cubedaw_lib::{Id, IdMap, Note, Section, State, Track};

use crate::util::{DragHandler, SelectableUiData};

#[derive(Debug)]
pub struct UiState {
    pub sections: IdMap<Section, SectionUiState>,
    pub notes: IdMap<Note, NoteUiState>,
    pub tracks: IdMap<Track, TrackUiState>,

    // An ordered track list. This is the order with which the tracks are displayed in the track tab.
    pub track_list: Vec<Id<Track>>,

    pub section_drag: DragHandler,
    pub note_drag: DragHandler,
}

impl UiState {
    pub fn track(&mut self, state: &State) {
        self.sections.track(&state.sections);
        self.tracks.track(&state.tracks);
        self.notes.track(&state.notes);

        for event in state.tracks.events().unwrap() {
            match event {
                cubedaw_lib::TrackingMapEvent::Create(id) => {
                    self.track_list.push(*id);
                }
                cubedaw_lib::TrackingMapEvent::Delete(id) => self.track_list.retain(|o| o != id),
            }
        }
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sections: IdMap::nontracking(),
            tracks: IdMap::nontracking(),
            notes: IdMap::nontracking(),

            track_list: Vec::new(),

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
    pub name: String,
    pub is_editing_name: bool,
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
