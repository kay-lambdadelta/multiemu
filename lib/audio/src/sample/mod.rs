use num::{FromPrimitive, Num, ToPrimitive, traits::NumAssignOps};
use std::fmt::Debug;

pub mod conversion;
pub mod iterator;

/// Base trait for samples
pub trait Sample:
    Num + NumAssignOps + PartialOrd + Debug + ToPrimitive + FromPrimitive + Copy + 'static
{
    /// Minimum sample value
    const SAMPLE_MIN: Self;

    /// Maximum sample value
    const SAMPLE_MAX: Self;

    /// The midpoint of the sample range
    fn equilibrium() -> Self {
        (Self::SAMPLE_MIN + Self::SAMPLE_MAX) / (Self::one() + Self::one())
    }

    /// Clamps the sample, should be done after every set of operations
    fn normalize(self) -> Self {
        num::clamp(self, Self::SAMPLE_MIN, Self::SAMPLE_MAX)
    }
}

/// Automatic sample implementation macro
macro_rules! sample_impl {
    (float, $sample:ty) => {
        impl Sample for $sample {
            const SAMPLE_MIN: Self = -1.0;
            const SAMPLE_MAX: Self = 1.0;
        }
    };

    (int, $sample:ty) => {
        impl Sample for $sample {
            const SAMPLE_MIN: Self = Self::MIN;
            const SAMPLE_MAX: Self = Self::MAX;
        }
    };
}

sample_impl!(float, f32);
sample_impl!(float, f64);
sample_impl!(int, i8);
sample_impl!(int, i16);
sample_impl!(int, i32);
sample_impl!(int, i64);
sample_impl!(int, u8);
sample_impl!(int, u16);
sample_impl!(int, u32);
sample_impl!(int, u64);

// TODO: Implement on nums Ratio?
