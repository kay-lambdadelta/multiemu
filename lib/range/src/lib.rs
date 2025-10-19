#![no_std]

mod range_inclusive;
mod range_inclusive_set;

pub trait RangeBase<Idx> {
    fn is_empty(&self) -> bool;
}

pub trait ContiguousRange<Idx>: RangeBase<Idx> {
    fn from_start_and_length(start: Idx, length: Idx) -> Self;
    fn is_adjacent(&self, other: &Self) -> bool;
    fn len(&self) -> usize;
}

pub trait RangeIntersection<Idx, Rhs: RangeBase<Idx> = Self>: RangeBase<Idx> {
    type Output: RangeBase<Idx>;

    fn intersects(&self, rhs: &Rhs) -> bool;
    fn intersection(&self, rhs: &Rhs) -> Self::Output;

    fn disjoint(&self, rhs: &Rhs) -> bool {
        !self.intersects(rhs)
    }
}

pub trait RangeDifference<Idx, Rhs: RangeBase<Idx> = Self>: RangeBase<Idx> {
    type Output: RangeBase<Idx>;

    fn difference(&self, rhs: &Rhs) -> Self::Output;
}
