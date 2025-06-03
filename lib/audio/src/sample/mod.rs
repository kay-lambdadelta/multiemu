use num::{
    Bounded, FromPrimitive, Integer, Num, ToPrimitive, rational::Ratio, traits::NumAssignOps,
};
use std::fmt::Debug;

pub mod conversion;
pub mod iterator;

/// Base trait for samples
pub trait Sample:
    Num + NumAssignOps + PartialOrd + Debug + ToPrimitive + FromPrimitive + Copy + Send + Sync + 'static
{
    fn sample_min() -> Self;

    fn sample_max() -> Self;

    /// The midpoint of the sample range
    fn equilibrium() -> Self {
        (Self::sample_min() + Self::sample_max()) / (Self::one() + Self::one())
    }

    /// Clamps the sample, should be done after every set of operations
    fn normalize(self) -> Self {
        num::clamp(self, Self::sample_min(), Self::sample_max())
    }
}

/// Automatic sample implementation macro
macro_rules! sample_impl {
    (float, $sample:ty) => {
        impl Sample for $sample {
            fn sample_min() -> Self {
                -1.0
            }

            fn sample_max() -> Self {
                1.0
            }
        }
    };

    (int, $sample:ty) => {
        impl Sample for $sample {
            fn sample_min() -> Self {
                Self::MIN
            }

            fn sample_max() -> Self {
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
> Sample for Ratio<R>
where
    Ratio<R>: ToPrimitive + FromPrimitive,
{
    fn sample_min() -> Self {
        Ratio::from_integer(R::min_value())
    }

    fn sample_max() -> Self {
        Ratio::from_integer(R::max_value())
    }
}
