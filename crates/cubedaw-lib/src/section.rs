use std::collections::BTreeMap;

use meminterval::IntervalTree;

use crate::{Id, IdMap, Note, Range};

#[derive(Clone, Debug)]
/// A section on a track, independent of a start position
pub struct Section {
    pub name: String,
    pub length: u64,

    note_map: IdMap<Note, (i64, Note)>,
    notes_range: IntervalTree<i64, Id<Note>>,
    notes_start_position: BTreeMap<i64, Id<Note>>,
}

impl Section {
    pub fn empty(name: String, length: u64) -> Self {
        Self {
            name,
            length,

            note_map: IdMap::new(),
            notes_range: IntervalTree::new(),
            notes_start_position: BTreeMap::new(),
        }
    }

    pub fn insert_note(&mut self, start_pos: i64, note_id: Id<Note>, note: Note) {
        self.notes_range.insert(note.range_with(start_pos), note_id);
        self.notes_start_position.insert(start_pos, note_id);
        self.note_map.insert(note_id, (start_pos, note));
    }

    pub fn remove_note(&mut self, note_id: Id<Note>) -> (i64, Note) {
        let (start_pos, note) = self.note_map.take(note_id);
        self.notes_range.delete(note.range_with(start_pos));
        let removed = self.notes_start_position.remove(&start_pos);
        debug_assert!(
            removed.is_some(),
            "notes_start_position desynced with note_map"
        );
        (start_pos, note)
    }

    pub fn move_note(&mut self, note_id: Id<Note>, pos_offset: i64, pitch_offset: i32) {
        let (start_pos, note) = self
            .note_map
            .get_mut(note_id)
            .expect("note in self.notes but not in state.notes????");

        self.notes_range.delete(note.range_with(*start_pos));
        self.notes_start_position.remove(start_pos);
        note.pitch += pitch_offset;
        *start_pos += pos_offset;
        self.notes_range
            .insert(note.range_with(*start_pos), note_id);
        self.notes_start_position.insert(*start_pos, note_id);
    }

    pub fn note(&self, id: Id<Note>) -> Option<(i64, &Note)> {
        self.note_map.get(id).map(|(id, note)| (*id, note))
    }
    pub fn note_mut(&mut self, id: Id<Note>) -> Option<(i64, &mut Note)> {
        self.note_map.get_mut(id).map(|(id, note)| (*id, note))
    }

    pub fn notes_intersecting(&self, range: Range) -> impl Iterator<Item = (i64, Id<Note>, &Note)> {
        self.notes_range.query(range).map(|entry| {
            (
                entry.interval.start,
                *entry.value,
                &self
                    .note_map
                    .get(*entry.value)
                    .expect("notes_range desynced with note_map")
                    .1,
            )
        })
    }
    pub fn notes(&self) -> impl Iterator<Item = (i64, Id<Note>, &Note)> {
        self.notes_start_position
            .iter()
            .map(|(&start_pos, &note_id)| {
                (
                    start_pos,
                    note_id,
                    &self
                        .note_map
                        .get(note_id)
                        .expect("notes_start_position desynced with note_map")
                        .1,
                )
            })
    }

    pub fn note_start_positions_in(
        &self,
        range: Range,
    ) -> impl Iterator<Item = (i64, Id<Note>, &Note)> {
        self.notes_start_position
            .range(range.start..range.end)
            .map(|(&start_pos, &note_id)| {
                (
                    start_pos,
                    note_id,
                    &self
                        .note_map
                        .get(note_id)
                        .expect("notes_start_position desynced with note_map")
                        .1,
                )
            })
    }
}
