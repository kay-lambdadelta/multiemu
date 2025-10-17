use crate::RangeBase;
use num::Integer;
use rangemap::RangeInclusiveSet;
use rangemap::StepLite;

impl<Idx: Integer + Clone + StepLite> RangeBase<Idx> for RangeInclusiveSet<Idx> {
    fn is_empty(&self) -> bool {
        self.is_empty()
    }
}
