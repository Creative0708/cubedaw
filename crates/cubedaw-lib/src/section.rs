use std::collections::BTreeSet;

use crate::{Id, IdMap, Note};

#[derive(Clone, Debug)]
/// A section on a track, independent of a start position
pub struct Section {
    pub name: String,
    pub length: u64,

    // notes can share the same position, so this allows that by ordering by Ids if the Ranges match
    notes: BTreeSet<(i64, Id<Note>)>,
}

impl Section {
    pub fn empty(name: String, length: u64) -> Self {
        Self {
            name,
            length,
            notes: BTreeSet::new(),
        }
    }

    pub fn insert_note(
        &mut self,
        notes: &mut IdMap<Note>,
        start_pos: i64,
        note_id: Id<Note>,
        note: Note,
    ) {
        notes.set(note_id, note);
        self.notes.insert((start_pos, note_id));
    }

    pub fn remove_note(
        &mut self,
        notes: &mut IdMap<Note>,
        start_pos: i64,
        note_id: Id<Note>,
    ) -> Note {
        let note = notes
            .remove(note_id)
            .expect("tried to remove nonexistent note");
        if !self.notes.remove(&(start_pos, note_id)) {
            panic!("note in state.notes but not in internal note map");
        }
        note
    }

    pub fn move_note(
        &mut self,
        notes: &mut IdMap<Note>,
        note_start: i64,
        note_id: Id<Note>,
        new_start: i64,
        pitch_offset: i32,
    ) {
        if !self.notes.remove(&(note_start, note_id)) {
            panic!("Tried to remove nonexistent note at {note_start:?}: {note_id:?}");
        }
        let note = notes
            .get_mut(note_id)
            .expect("note in self.notes but not in state.notes????");
        note.pitch += pitch_offset;
        self.notes.insert((new_start, note_id));
    }

    pub fn notes(&self) -> impl Iterator<Item = (i64, Id<Note>)> + '_ {
        self.notes.iter().copied()
    }
}
