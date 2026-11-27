//! Half-open validity intervals in astronomical years (1 BC = 0).

/// Validity `[from, to)` in astronomical years; always non-empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValidityInterval {
    from: i32,
    to: i32,
}

impl ValidityInterval {
    /// Creates `[from, to)`; `None` when the interval would be empty.
    pub const fn new(from: i32, to: i32) -> Option<Self> {
        if from < to {
            Some(Self { from, to })
        } else {
            None
        }
    }

    /// First year (inclusive).
    pub const fn from(self) -> i32 {
        self.from
    }

    /// End year (exclusive).
    pub const fn to(self) -> i32 {
        self.to
    }

    /// `true` when `year` lies inside the interval.
    pub const fn contains(self, year: i32) -> bool {
        self.from <= year && year < self.to
    }

    /// `true` when both intervals share at least one year.
    pub const fn overlaps(self, other: Self) -> bool {
        self.from < other.to && other.from < self.to
    }

    /// `true` when `other` lies completely inside `self`.
    pub const fn covers(self, other: Self) -> bool {
        self.from <= other.from && other.to <= self.to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn half_open_semantics() {
        let i = ValidityInterval::new(476, 1453).unwrap();
        assert!(i.contains(476));
        assert!(i.contains(1452));
        assert!(!i.contains(1453));
        assert!(!i.contains(475));
    }

    #[test]
    fn empty_interval_rejected() {
        assert!(ValidityInterval::new(10, 10).is_none());
        assert!(ValidityInterval::new(10, 9).is_none());
    }

    #[test]
    fn overlap_and_cover() {
        let a = ValidityInterval::new(0, 100).unwrap();
        let b = ValidityInterval::new(100, 200).unwrap();
        let c = ValidityInterval::new(50, 60).unwrap();
        assert!(!a.overlaps(b), "adjacent intervals do not overlap");
        assert!(a.overlaps(c));
        assert!(a.covers(c));
        assert!(!c.covers(a));
    }
}
