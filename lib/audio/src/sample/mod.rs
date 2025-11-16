use core::fmt::Debug;

use num::{
    Bounded, FromPrimitive, Integer, Num, ToPrimitive, rational::Ratio, traits::NumAssignOps,
};

mod conversion;
mod iterator;

pub use conversion::*;
pub use iterator::SampleIterator;

/// Base trait for samples
pub trait SampleFormat:
    Num + NumAssignOps + PartialOrd + Debug + ToPrimitive + FromPrimitive + Copy + Send + Sync + 'static
{
    /// The minimum value of the sample
    fn min_sample() -> Self;

    /// The maximum value of the sample
    fn max_sample() -> Self;

    /// The midpoint of the sample range
    fn equilibrium() -> Self {
        (Self::min_sample() + Self::max_sample()) / (Self::one() + Self::one())
    }

    /// Clamps the sample, should be done after every set of operations
    fn normalize(self) -> Self {
        num::clamp(self, Self::min_sample(), Self::max_sample())
    }
}

/// Automatic sample implementation macro
macro_rules! sample_impl {
    (float, $sample:ty) => {
        impl SampleFormat for $sample {
            fn min_sample() -> Self {
                -1.0
            }

            fn max_sample() -> Self {
                1.0
            }
        }
    };

    (int, $sample:ty) => {
        impl SampleFormat for $sample {
            fn min_sample() -> Self {
                Self::MIN
            }

            fn max_sample() -> Self {
                Self::MAX
            }
        }
    };
}

sample_impl!(int, i8);
sample_impl!(int, i16);
sample_impl!(int, i32);
sample_impl!(int, i64);
sample_impl!(int, i128);
sample_impl!(int, u8);
sample_impl!(int, u16);
sample_impl!(int, u32);
sample_impl!(int, u64);
sample_impl!(int, u128);
sample_impl!(float, f32);
sample_impl!(float, f64);

impl<
    R: Integer
        + NumAssignOps
        + Debug
        + ToPrimitive
        + FromPrimitive
        + Copy
        + Send
        + Sync
        + Bounded
        + 'static,
> SampleFormat for Ratio<R>
where
    Ratio<R>: ToPrimitive + FromPrimitive,
{
    fn min_sample() -> Self {
        Ratio::from_integer(R::min_value())
    }

    fn max_sample() -> Self {
        Ratio::from_integer(R::max_value())
    }
}
