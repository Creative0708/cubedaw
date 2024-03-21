use crate::Range;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug, Default)]
pub struct Note {
    pub range: Range,
    // Logarithmic pitch. Middle C (261.626 Hz) == 0, so in 12TET C# == 1, E == 4, etc.
    pub pitch: i32,
}

impl Note {
    #[inline]
    pub fn from_range_pitch(range: Range, pitch: i32) -> Self {
        Self { range, pitch }
    }

    #[inline]
    pub fn start(&self) -> i64 {
        self.range.start
    }
    #[inline]
    pub fn end(&self) -> i64 {
        self.range.end
    }
    #[inline]
    pub fn start_mut(&mut self) -> &mut i64 {
        &mut self.range.start
    }
    #[inline]
    pub fn end_mut(&mut self) -> &mut i64 {
        &mut self.range.end
    }
}
