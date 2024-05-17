use crate::Range;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Default)]
/// A struct representing a note independent of start position.
pub struct Note {
    pub length: u64,

    // Logarithmic pitch. Middle C (261.626 Hz) == 0, so in 12TET C# == 1, E == 4, etc.
    pub pitch: i32,
}

impl Note {
    pub fn new(length: u64, pitch: i32) -> Self {
        Self { length, pitch }
    }

    pub fn range_with(&self, start_pos: i64) -> Range {
        Range::new(
            start_pos,
            start_pos
                .checked_add_unsigned(self.length)
                .expect("start + length overflows i64"),
        )
    }
}
