use std::ops;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Default)]
/// cubedaw range. Describes an inclusive start position and an exclusive end position.
pub struct Range {
    pub start: i64,
    pub end: i64,
}

impl Range {
    pub const UNITS_PER_BEAT: u64 = 256;
    pub const EMPTY: Self = Range { start: 0, end: 0 };
    pub const EVERYTHING: Self = Range {
        start: i64::MIN,
        end: i64::MAX,
    };

    pub fn new(start: i64, end: i64) -> Self {
        Self { start, end }
    }
    pub fn from_start_length(start: i64, length: u64) -> Self {
        Self {
            start,
            end: start + length as i64,
        }
    }
    pub fn from_start_length_signed(start: i64, length: i64) -> Self {
        Self {
            start,
            end: start + length,
        }
    }
    pub fn from_beats(start: i64, end: i64) -> Self {
        Self {
            start: start * Self::UNITS_PER_BEAT as i64,
            end: end * Self::UNITS_PER_BEAT as i64,
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
        let start =
            div_floor(pos, Self::UNITS_PER_BEAT as i64 * 4) * (Self::UNITS_PER_BEAT as i64 * 4);
        Self {
            start,
            end: start + Self::UNITS_PER_BEAT as i64 * 4,
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

    pub fn with_start_pos(self, start: i64) -> Self {
        Self::from_start_length_signed(start, self.length())
    }

    pub fn length(self) -> i64 {
        self.end - self.start
    }
    pub fn valid(self) -> bool {
        self.length() >= 0
    }

    pub fn contains(self, pos: i64) -> bool {
        pos >= self.start && pos < self.end
    }

    pub fn intersect(self, other: Self) -> Self {
        Self {
            start: self.start.max(other.start),
            end: self.end.min(other.end),
        }
    }
    pub fn intersects(self, other: Self) -> bool {
        self.start < other.end && self.end > other.start
    }

    pub fn iter_snap_to(self, snap: i64) -> impl Iterator<Item = i64> {
        self.multiples_within_range(snap).map(move |x| x * snap)
    }
    pub fn multiples_within_range(self, snap: i64) -> std::ops::RangeInclusive<i64> {
        let start = div_ceil(self.start, snap);
        let end = div_floor(self.end, snap);
        start..=end
    }
}

// copied from rust std
const fn div_ceil(this: i64, rhs: i64) -> i64 {
    let d = this / rhs;
    let r = this % rhs;

    // When remainder is non-zero we have a.div_ceil(b) == 1 + a.div_floor(b),
    // so we can re-use the algorithm from div_floor, just adding 1.
    let correction = 1 + ((this ^ rhs) >> (i64::BITS - 1));
    if r != 0 { d + correction } else { d }
}
const fn div_floor(this: i64, rhs: i64) -> i64 {
    let d = this / rhs;
    let r = this % rhs;

    // If the remainder is non-zero, we need to subtract one if the
    // signs of self and rhs differ, as this means we rounded upwards
    // instead of downwards. We do this branchlessly by creating a mask
    // which is all-ones iff the signs differ, and 0 otherwise. Then by
    // adding this mask (which corresponds to the signed value -1), we
    // get our correction.
    let correction = (this ^ rhs) >> (i64::BITS - 1);
    if r != 0 { d + correction } else { d }
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
