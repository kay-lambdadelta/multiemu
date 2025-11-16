use core::ops::RangeInclusive;

use num::{Integer, ToPrimitive};
use rangemap::{RangeInclusiveSet, StepLite};

use crate::{ContiguousRange, RangeBase, RangeDifference, RangeIntersection};

impl<Idx: Integer + Clone> RangeBase<Idx> for RangeInclusive<Idx> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}

impl<Idx: Integer + Clone + ToPrimitive> ContiguousRange<Idx> for RangeInclusive<Idx> {
    #[inline]
    fn from_start_and_length(start: Idx, length: Idx) -> Self {
        let one = Idx::one();

        start.clone()..=(start + length - one)
    }

    #[inline]
    fn is_adjacent(&self, other: &Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && (self.end().clone() + Idx::one() == other.start().clone()
                || other.end().clone() + Idx::one() == self.start().clone())
    }

    #[inline]
    fn len(&self) -> usize {
        if self.is_empty() {
            return 0;
        }

        let start = self.start().to_usize().unwrap();
        let end = self.end().to_usize().unwrap();

        end - start + 1
    }
}

impl<Idx: Integer + Clone> RangeIntersection<Idx, Self> for RangeInclusive<Idx> {
    type Output = RangeInclusive<Idx>;

    #[inline]
    fn intersection(&self, rhs: &Self) -> Self::Output {
        let start = core::cmp::max(self.start(), rhs.start()).clone();
        let end = core::cmp::min(self.end(), rhs.end()).clone();

        start..=end
    }

    #[inline]
    fn intersects(&self, rhs: &Self) -> bool {
        !self.intersection(rhs).is_empty()
    }
}

impl<Idx: Integer + Clone + StepLite> RangeDifference<Idx, Self> for RangeInclusive<Idx> {
    type Output = RangeInclusiveSet<Idx>;

    #[inline]
    fn difference(&self, rhs: &Self) -> Self::Output {
        if self.is_empty() {
            return RangeInclusiveSet::default();
        }

        if rhs.end() < self.start() || rhs.start() > self.end() {
            return RangeInclusiveSet::from_iter([self.clone()]);
        }

        let mut result = RangeInclusiveSet::default();
        let one = Idx::one();

        if rhs.start() > self.start() {
            let left_end = rhs.start().clone() - one.clone();

            if &left_end >= self.start() {
                result.insert(self.start().clone()..=left_end);
            }
        }

        if rhs.end() < self.end() {
            let right_start = rhs.end().clone() + one;

            if &right_start <= self.end() {
                result.insert(right_start..=self.end().clone());
            }
        }

        result
    }
}

#[cfg(test)]
#[allow(clippy::reversed_empty_ranges)]
mod tests {
    use super::*;

    #[test]
    fn test_is_empty() {
        let r = 1..=5;
        assert!(!r.is_empty());

        let empty = 5..=4;
        assert!(empty.is_empty());
    }

    #[test]
    fn from_start_and_length() {
        let r = RangeInclusive::from_start_and_length(10, 5);
        assert_eq!(r, 10..=14);

        let r = RangeInclusive::from_start_and_length(7, 1);
        assert_eq!(r, 7..=7);

        let r = RangeInclusive::from_start_and_length(10, 0);
        assert!(r.is_empty());
    }

    #[test]
    fn test_intersection_overlapping() {
        let a = 0..=10;
        let b = 5..=15;
        let intersection = a.intersection(&b);
        assert_eq!(intersection, 5..=10);
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_intersection_disjoint() {
        let a = 0..=4;
        let b = 5..=10;
        let intersection = a.intersection(&b);
        assert!(intersection.is_empty());
        assert!(!a.intersects(&b));
    }

    #[test]
    fn test_intersection_contained() {
        let a = 0..=10;
        let b = 2..=5;
        let intersection = a.intersection(&b);
        assert_eq!(intersection, 2..=5);
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_intersection_identical() {
        let a = 3..=8;
        let b = 3..=8;
        let intersection = a.intersection(&b);
        assert_eq!(intersection, 3..=8);
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_intersection_empty_range() {
        let a = 0..=5;
        let b = 7..=6;
        assert!(b.is_empty());

        let inter = a.intersection(&b);
        assert!(inter.is_empty());
        assert!(!a.intersects(&b));
    }

    #[test]
    fn test_intersection_single_point() {
        let a = 0..=5;
        let b = 5..=10;
        let inter = a.intersection(&b);
        assert_eq!(inter, 5..=5);
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_intersection_negative_numbers() {
        let a = (-10)..=(-1);
        let b = (-5)..=5;
        let inter = a.intersection(&b);
        assert_eq!(inter, (-5)..=(-1));
        assert!(a.intersects(&b));
    }

    #[test]
    fn test_disjoint_basic() {
        let a = 0..=4;
        let b = 5..=10;
        assert!(a.disjoint(&b));
        assert!(b.disjoint(&a));

        let a = 0..=5;
        let b = 5..=10;
        assert!(!a.disjoint(&b));
        assert!(!b.disjoint(&a));

        let a = 0..=10;
        let b = 8..=15;
        assert!(!a.disjoint(&b));

        let a = 0..=10;
        let b = 3..=7;
        assert!(!a.disjoint(&b));

        let a = 3..=8;
        let b = 3..=8;
        assert!(!a.disjoint(&b));

        let a = 5..=4; // empty
        let b = 0..=10;
        assert!(a.is_empty());
        assert!(a.disjoint(&b));
        assert!(b.disjoint(&a));

        let a = (-10)..=(-5);
        let b = (-4)..=0;
        assert!(a.disjoint(&b));

        let a = (-10)..=(-5);
        let b = (-5)..=0;
        assert!(!a.disjoint(&b));
    }
}
