use std::collections::BTreeMap;

use crate::{Note, Range};

#[derive(Clone, Debug)]
pub struct Section {
    pub range: Range,

    // Notes sorted by starting position
    notes: BTreeMap<i64, Note>,
}

impl Section {
    pub fn empty(range: Range) -> Self {
        Self {
            range,
            notes: BTreeMap::new(),
        }
    }

    pub fn insert_note(&mut self, note: Note) {
        self.notes.insert(note.range.start, note);
    }

    pub fn notes(&self) -> impl Iterator<Item = &Note> {
        self.notes.values()
    }
    pub fn notes_mut(&mut self) -> impl Iterator<Item = &mut Note> {
        self.notes.values_mut()
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
