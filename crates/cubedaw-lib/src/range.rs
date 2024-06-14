use std::ops;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default)]
pub struct Range {
    pub start: i64,
    pub end: i64,
}

impl Range {
    pub const UNITS_PER_BEAT: i64 = 256;
    pub const EMPTY: Self = Range { start: 0, end: 0 };
    pub const EVERYTHING: Self = Range {
        start: i64::MIN,
        end: i64::MAX,
    };

    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }
    pub fn start_length(start: i64, length: u64) -> Self {
        Self {
            start,
            end: start
                .checked_add_unsigned(length)
                .expect("i64 + u64 overflow"),
        }
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
    pub fn at(pos: i64) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
    pub fn surrounding_pos(pos: i64) -> Self {
        // TODO replace with i64::div_floor when it's stabilized
        pub const fn div_floor(lhs: i64, rhs: i64) -> i64 {
            let d = lhs / rhs;
            let r = lhs % rhs;
            if (r > 0 && rhs < 0) || (r < 0 && rhs > 0) {
                d - 1
            } else {
                d
            }
        }
        let start = div_floor(pos, Self::UNITS_PER_BEAT * 4) * (Self::UNITS_PER_BEAT * 4);
        Self {
            start,
            end: start + Self::UNITS_PER_BEAT * 4,
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

impl ops::Add<i64> for Range {
    type Output = Self;
    fn add(self, rhs: i64) -> Self::Output {
        Self {
            start: self.start + rhs,
            end: self.end + rhs,
        }
    }
}

impl ops::AddAssign<i64> for Range {
    fn add_assign(&mut self, rhs: i64) {
        *self = *self + rhs;
    }
}

impl ops::Sub<i64> for Range {
    type Output = Self;
    fn sub(self, rhs: i64) -> Self::Output {
        self + -rhs
    }
}

impl ops::SubAssign<i64> for Range {
    fn sub_assign(&mut self, rhs: i64) {
        *self = *self - rhs;
    }
}

impl From<Range> for meminterval::Interval<i64> {
    fn from(value: Range) -> Self {
        Self::new(value.start, value.end)
    }
}
impl From<meminterval::Interval<i64>> for Range {
    fn from(value: meminterval::Interval<i64>) -> Self {
        Self::new(value.start, value.end)
    }
}
