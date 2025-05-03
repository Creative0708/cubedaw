use cubedaw_lib::{Id, Note, Clip, Track};

use crate::StateCommand;

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
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        if let Some(clip) = self.clip(state) {
            clip.move_note(self.note_id, self.pos_offset, self.pitch_offset);
        }
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        if let Some(clip) = self.clip(state) {
            clip.move_note(self.note_id, -self.pos_offset, -self.pitch_offset);
        }
    }
}

// TODO see TrackAddOrRemove
#[derive(Clone)]
pub struct NoteAddOrRemove {
    id: Id<Note>,
    start_pos: i64,
    track_id: Id<Track>,
    clip_id: Id<Clip>,
    data: Option<Note>,
    is_removal: bool,
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

    pub fn track_id(&self) -> Id<Track> {
        self.track_id
    }
    pub fn clip_id(&self) -> Id<Clip> {
        self.clip_id
    }
    pub fn id(&self) -> Id<Note> {
        self.id
    }
    pub fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn clip<'a>(&self, state: &'a mut cubedaw_lib::State) -> Option<&'a mut cubedaw_lib::Clip> {
        Some(
            state
                .tracks
                .get_mut(self.track_id)?
                .clip_mut(self.clip_id)?,
        )
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        let note_data = self
            .data
            .take()
            .expect("called execute_add on empty NoteAddOrRemove");

        if let Some(clip) = self.clip(state) {
            clip.insert_note(self.start_pos, self.id, note_data);
        }
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        if let Some(clip) = self.clip(state) {
            let (start_pos, note_data) = clip.remove_note(self.id);

            if self.data.replace(note_data).is_some() {
                panic!("called execute_remove on nonempty NoteAddOrRemove");
            }
            self.start_pos = start_pos;
        }
    }
}

impl StateCommand for NoteAddOrRemove {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        if self.is_removal {
            self.execute_remove(state);
        } else {
            self.execute_add(state);
        }
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        if self.is_removal {
            self.execute_add(state);
        } else {
            self.execute_remove(state);
        }
    }
}
