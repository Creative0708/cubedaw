use meminterval::IntervalTree;

use crate::{Id, IdMap, Note, Range};

#[derive(Clone, Debug)]
/// A section on a track, independent of a start position
pub struct Section {
    pub name: String,
    pub length: u64,

    // an invariant is that there is an entry for a note in note_map iff there is one in notes.
    // TODO possibly convert to unsafe for optimizations???
    note_map: IdMap<Note, (i64, Note)>,

    notes: IntervalTree<i64, Id<Note>>,
}

impl Section {
    pub fn empty(name: String, length: u64) -> Self {
        Self {
            name,
            length,

            note_map: IdMap::new(),

            notes: IntervalTree::new(),
        }
    }

    pub fn insert_note(&mut self, start_pos: i64, note_id: Id<Note>, note: Note) {
        self.notes.insert(note.range_with(start_pos), note_id);
        self.note_map.insert(note_id, (start_pos, note));
    }

    pub fn remove_note(&mut self, note_id: Id<Note>) -> (i64, Note) {
        let (start_pos, note) = self.note_map.take(note_id);
        self.notes.delete(note.range_with(start_pos));
        (start_pos, note)
    }

    pub fn move_note(&mut self, note_id: Id<Note>, pos_offset: i64, pitch_offset: i32) {
        dbg!(note_id, pos_offset, pitch_offset);
        let (pos, note) = self
            .note_map
            .get_mut(note_id)
            .expect("note in self.notes but not in state.notes????");

        self.notes.delete(note.range_with(*pos));
        note.pitch += pitch_offset;
        *pos += pos_offset;
        self.notes.insert(note.range_with(*pos), note_id);
    }

    pub fn note(&self, id: Id<Note>) -> Option<(i64, &Note)> {
        self.note_map.get(id).map(|(id, note)| (*id, note))
    }
    pub fn note_mut(&mut self, id: Id<Note>) -> Option<(i64, &mut Note)> {
        self.note_map.get_mut(id).map(|(id, note)| (*id, note))
    }

    pub fn notes_intersecting(&self, range: Range) -> impl Iterator<Item = (i64, Id<Note>, &Note)> {
        self.notes.query(range).map(|entry| {
            (
                entry.interval.start,
                *entry.value,
                &self
                    .note_map
                    .get(*entry.value)
                    .expect("id in self.notes but not in self.note_map???")
                    .1,
            )
        })
    }
    pub fn notes(&self) -> impl Iterator<Item = (i64, Id<Note>, &Note)> {
        // TODO is there _really_ not a better way?
        self.notes_intersecting(Range::EVERYTHING)
    }
}
