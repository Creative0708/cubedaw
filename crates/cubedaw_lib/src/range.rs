#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default)]
pub struct Range {
    pub start: i64,
    pub end: i64,
}

impl Range {
    pub const UNITS_PER_BEAT: i64 = 256;
    pub const EMPTY: Self = Range { start: 0, end: 0 };

    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }
    pub fn from_beats(start: i64, end: i64) -> Self {
        Self {
            start: start * Self::UNITS_PER_BEAT,
            end: end * Self::UNITS_PER_BEAT,
        }
    }
    pub fn from_range(range: std::ops::Range<i64>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
    pub fn new_at(pos: i64) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
    pub fn surrounding_pos(pos: i64) -> Self {
        let start = pos.div_floor(Self::UNITS_PER_BEAT) * Self::UNITS_PER_BEAT;
        Self {
            start,
            end: start + Self::UNITS_PER_BEAT,
        }
    }
    pub fn unbounded_start(end: i64) -> Self {
        Self {
            start: i64::MIN,
            end,
        }
    }
    pub fn unbounded_end(start: i64) -> Self {
        Self {
            start,
            end: i64::MAX,
        }
    }

    pub fn length(&self) -> i64 {
        self.end - self.start
    }
    pub fn valid(&self) -> bool {
        self.length() >= 0
    }

    pub fn contains(&self, pos: i64) -> bool {
        pos >= self.start && pos < self.end
    }

    pub fn intersect(&self, other: Self) -> Self {
        Self {
            start: self.start.max(other.start),
            end: self.end.min(other.end),
        }
    }
    pub fn intersects(&self, other: Self) -> bool {
        self.start < other.end && self.end > other.start
    }
}
