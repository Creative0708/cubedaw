use std::collections::BTreeSet;

use crate::{Note, Range};

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub range: Range,

    // Notes sorted by starting position
    notes: BTreeSet<Note>,
}

impl Section {
    pub fn empty(name: String, range: Range) -> Self {
        Self {
            name,
            range,
            notes: BTreeSet::new(),
        }
    }

    pub fn insert_note(&mut self, mut note: Note) {
        note.range -= self.start();
        self.notes.insert(note);
    }

    pub fn notes(&self) -> impl Iterator<Item = &Note> {
        self.notes.iter()
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
