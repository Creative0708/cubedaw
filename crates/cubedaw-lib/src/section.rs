use std::collections::BTreeSet;

use crate::{Id, IdMap, Note, Range};

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub range: Range,

    // notes can share the exact same Range, so this allows that by ordering by Ids if the Ranges match
    notes: BTreeSet<(Range, Id<Note>)>,
}

impl Section {
    pub fn empty(name: String, range: Range) -> Self {
        Self {
            name,
            range,
            notes: BTreeSet::new(),
        }
    }

    pub fn insert_note(&mut self, notes: &mut IdMap<Note>, mut note: Note) -> Id<Note> {
        note.range -= self.start();
        let note_range = note.range;
        let note_id = notes.create(note);
        self.notes.insert((note_range, note_id));
        note_id
    }

    pub fn move_note(
        &mut self,
        notes: &mut IdMap<Note>,
        note_range: Range,
        note_id: Id<Note>,
        new_range: Range,
        pitch_offset: i32,
    ) {
        if !self.notes.remove(&(note_range, note_id)) {
            panic!("Tried to remove nonexistent note at {note_range:?}: {note_id:?}");
        }
        let note = notes.get_mut(note_id);
        note.range = new_range;
        note.pitch += pitch_offset;
        self.notes.insert((new_range, note_id));
    }

    pub fn notes(&self) -> impl Iterator<Item = (Range, Id<Note>)> + '_ {
        self.notes.iter().map(|(r, n)| (*r, *n))
    }

    #[inline]
    pub fn start(&self) -> i64 {
        self.range.start
    }
    #[inline]
    pub fn end(&self) -> i64 {
        self.range.end
    }
}
