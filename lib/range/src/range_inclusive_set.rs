use num::Integer;
use rangemap::{RangeInclusiveSet, StepLite};

use crate::RangeBase;

impl<Idx: Integer + Clone + StepLite> RangeBase<Idx> for RangeInclusiveSet<Idx> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}
