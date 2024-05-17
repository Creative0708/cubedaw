use cubedaw_lib::{Id, Note, Section};

use crate::StateCommand;

pub struct NoteMove {
    section_id: Id<Section>,
    note_id: Id<Note>,
    starting_pos: i64,
    new_pos: i64,
    pitch_offset: i32,
}

impl NoteMove {
    pub fn new(
        section_id: Id<Section>,
        note_id: Id<Note>,
        starting_pos: i64,
        new_pos: i64,
        pitch_offset: i32,
    ) -> Self {
        Self {
            section_id,
            note_id,
            starting_pos,
            new_pos,
            pitch_offset,
        }
    }
}

impl StateCommand for NoteMove {
    fn execute(&mut self, state: &mut cubedaw_lib::State) {
        let section = state
            .sections
            .get_mut(self.section_id)
            .expect("nonexistent section id in NoteMove");

        section.move_note(
            &mut state.notes,
            self.starting_pos,
            self.note_id,
            self.new_pos,
            self.pitch_offset,
        );
    }
    fn rollback(&mut self, state: &mut cubedaw_lib::State) {
        let section = state
            .sections
            .get_mut(self.section_id)
            .expect("nonexistent section id in NoteMove");

        section.move_note(
            &mut state.notes,
            self.new_pos,
            self.note_id,
            self.starting_pos,
            -self.pitch_offset,
        );
    }
}

// TODO see TrackAddOrRemove
pub struct NoteAddOrRemove {
    id: Id<Note>,
    start_pos: i64,
    section_id: Id<Section>,
    data: Option<Note>,
    is_removal: bool,
}

impl NoteAddOrRemove {
    pub fn addition(id: Id<Note>, start_pos: i64, data: Note, section_id: Id<Section>) -> Self {
        Self {
            id,
            start_pos,
            section_id,
            data: Some(data),
            is_removal: false,
        }
    }
    pub fn removal(id: Id<Note>, start_pos: i64, section_id: Id<Section>) -> Self {
        Self {
            id,
            start_pos,
            section_id,
            data: None,
            is_removal: true,
        }
    }

    pub fn id(&self) -> Id<Note> {
        self.id
    }
    pub fn is_removal(&self) -> bool {
        self.is_removal
    }

    fn execute_add(&mut self, state: &mut cubedaw_lib::State) {
        let note_data = self
            .data
            .take()
            .expect("called execute_add on empty NoteAddOrRemove");

        state
            .sections
            .get_mut(self.section_id)
            .expect("tried to add note to nonexistent section")
            .insert_note(&mut state.notes, self.start_pos, self.id, note_data);
    }
    fn execute_remove(&mut self, state: &mut cubedaw_lib::State) {
        let note_data = state
            .sections
            .get_mut(self.section_id)
            .expect("tried to remove note from nonexistent section")
            .remove_note(&mut state.notes, self.start_pos, self.id);

        if self.data.replace(note_data).is_none() {
            panic!("called execute_remove on nonempty NoteAddOrRemove");
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
