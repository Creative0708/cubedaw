use cubedaw_lib::{Clip, Id, Note, Track};
use cubedaw_worker::command::{ActionType, StateCommand, StateCommandWrapper};

use crate::{state::ui::NoteUiState, util::Select};

use super::UiStateCommand;

#[derive(Clone)]
pub struct NoteMove {
    track_id: Id<Track>,
    clip_id: Id<Clip>,
    note_id: Id<Note>,
    pos_offset: i64,
    pitch_offset: i32,
}

impl NoteMove {
    pub fn new(
        track_id: Id<Track>,
        clip_id: Id<Clip>,
        note_id: Id<Note>,
        time_offset: i64,
        pitch_offset: i32,
    ) -> Self {
        Self {
            track_id,
            clip_id,
            note_id,
            pos_offset: time_offset,
            pitch_offset,
        }
    }

    fn clip<'a>(&self, state: &'a mut cubedaw_lib::State) -> Option<&'a mut cubedaw_lib::Clip> {
        Some(
            state
                .tracks
                .get_mut(self.track_id)?
                .clip_mut(self.clip_id)?,
        )
    }
}

impl StateCommand for NoteMove {
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionType) {
        let Some(clip) = self.clip(state) else { return };
        match action {
            ActionType::Execute => {
                clip.move_note(self.note_id, self.pos_offset, self.pitch_offset);
            }
            ActionType::Rollback => {
                clip.move_note(self.note_id, -self.pos_offset, -self.pitch_offset);
            }
        }
    }
}

// TODO see TrackAddOrRemove
#[derive(Clone)]
struct NoUiNoteAddOrRemove {
    id: Id<Note>,
    start_pos: i64,
    track_id: Id<Track>,
    clip_id: Id<Clip>,
    data: Option<Note>,
    is_removal: bool,
}

impl NoUiNoteAddOrRemove {
    pub fn addition(
        id: Id<Note>,
        track_id: Id<Track>,
        clip_id: Id<Clip>,
        start_pos: i64,
        data: Note,
    ) -> Self {
        Self {
            id,
            start_pos,
            track_id,
            clip_id,
            data: Some(data),
            is_removal: false,
        }
    }
    pub fn removal(id: Id<Note>, track_id: Id<Track>, clip_id: Id<Clip>) -> Self {
        Self {
            id,
            start_pos: 0, // dummy value, will be replaced
            track_id,
            clip_id,
            data: None,
            is_removal: true,
        }
    }

    fn clip<'a>(&self, state: &'a mut cubedaw_lib::State) -> Option<&'a mut cubedaw_lib::Clip> {
        Some(
            state
                .tracks
                .get_mut(self.track_id)?
                .clip_mut(self.clip_id)?,
        )
    }
}

impl StateCommand for NoUiNoteAddOrRemove {
    fn run(&mut self, state: &mut cubedaw_lib::State, action: ActionType) {
        let Some(clip) = self.clip(state) else { return };
        if self.is_removal ^ action.is_rollback() {
            let (start_pos, note_data) = clip.remove_note(self.id);

            if self.data.replace(note_data).is_some() {
                panic!("called execute_remove on nonempty NoteAddOrRemove");
            }
            self.start_pos = start_pos;
        } else {
            let note_data = self
                .data
                .take()
                .expect("called execute_add on empty NoteAddOrRemove");

            clip.insert_note(self.start_pos, self.id, note_data);
        }
    }
}

pub struct NoteAddOrRemove {
    inner: NoUiNoteAddOrRemove,
    ui_data: Option<NoteUiState>,
}

impl NoteAddOrRemove {
    pub fn addition(
        id: Id<Note>,
        track_id: Id<Track>,
        clip_id: Id<Clip>,
        start_pos: i64,
        data: Note,
    ) -> Self {
        Self {
            inner: NoUiNoteAddOrRemove::addition(id, track_id, clip_id, start_pos, data),
            ui_data: None,
        }
    }
    pub fn removal(track_id: Id<Track>, clip_id: Id<Clip>, id: Id<Note>) -> Self {
        Self {
            inner: NoUiNoteAddOrRemove::removal(id, track_id, clip_id),
            ui_data: None,
        }
    }
}

impl UiStateCommand for NoteAddOrRemove {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
        action: ActionType,
    ) {
        let notes = &mut ui_state
            .tracks
            .force_get_mut(self.inner.track_id)
            .clips
            .force_get_mut(self.inner.clip_id)
            .notes;
        if self.inner.is_removal ^ action.is_rollback() {
            self.ui_data = notes.remove(self.inner.id);
            assert!(self.ui_data.is_some(), "tried to remove nonexistent note");
        } else {
            notes.insert(self.inner.id, self.ui_data.take().unwrap_or_default());
        }
    }
    fn inner(&mut self) -> Option<&mut dyn StateCommandWrapper> {
        Some(&mut self.inner)
    }
}

pub struct NoteSelect {
    track_id: Id<Track>,
    clip_id: Id<Clip>,
    id: Id<Note>,
    select: Select,
}

impl NoteSelect {
    pub fn new(track_id: Id<Track>, clip_id: Id<Clip>, id: Id<Note>, select: Select) -> Self {
        Self {
            track_id,
            clip_id,
            id,
            select,
        }
    }
}

impl UiStateCommand for NoteSelect {
    fn run_ui(
        &mut self,
        ui_state: &mut crate::UiState,
        _ephemeral_state: &mut crate::EphemeralState,
        action: ActionType,
    ) {
        let notes = &mut ui_state
            .tracks
            .force_get_mut(self.track_id)
            .clips
            .force_get_mut(self.clip_id)
            .notes;

        if let Some(ui_data) = notes.get_mut(self.id) {
            ui_data.select = self.select ^ action.is_rollback();
        }
    }
}
