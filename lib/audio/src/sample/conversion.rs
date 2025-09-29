use super::SampleFormat;

/// Conversion trait to get from one sample format to another
pub trait FromSample<T: SampleFormat>: SampleFormat {
    /// Converts from the target sample to our sample
    fn from_sample(sample: T) -> Self;
}

impl<T: SampleFormat> FromSample<T> for T {
    fn from_sample(sample: T) -> Self {
        sample.normalize()
    }
}

/// Reflexive version of [FromSample]
pub trait IntoSample<T: SampleFormat>: SampleFormat {
    /// Converts from our sample to the target sample
    fn into_sample(self) -> T;
}

impl<F: SampleFormat, I: FromSample<F>> IntoSample<I> for F {
    fn into_sample(self) -> I {
        I::from_sample(self)
    }
}

/// Automatic conversion macro
///
/// This is pretty efficient and accurate but could be made more efficient
macro_rules! conversion_impl {
    ($from:ty, $to:ty, $conversion_space:ty) => {
        impl FromSample<$from> for $to {
            #[inline]
            fn from_sample(sample: $from) -> Self {
                let from = sample as $conversion_space;
                let from_min = <$from>::min_sample() as $conversion_space;
                let from_max = <$from>::max_sample() as $conversion_space;

                let norm = (from - from_min) / (from_max - from_min);

                let to_min = <$to>::min_sample() as $conversion_space;
                let to_max = <$to>::max_sample() as $conversion_space;

                let scaled = norm * (to_max - to_min) + to_min;

                (scaled as $to).normalize()
            }
        }
    };
}

// unsigned to unsigned
conversion_impl!(u8, u16, u16);
conversion_impl!(u8, u32, u32);
conversion_impl!(u8, u64, u64);
conversion_impl!(u16, u8, u16);
conversion_impl!(u16, u32, u32);
conversion_impl!(u16, u64, u64);
conversion_impl!(u32, u8, u32);
conversion_impl!(u32, u16, u32);
conversion_impl!(u32, u64, u64);
conversion_impl!(u64, u8, u64);
conversion_impl!(u64, u16, u64);
conversion_impl!(u64, u32, u64);

// signed to signed
conversion_impl!(i8, i16, i32);
conversion_impl!(i8, i32, i64);
conversion_impl!(i8, i64, i128);
conversion_impl!(i16, i8, i32);
conversion_impl!(i16, i32, i64);
conversion_impl!(i16, i64, i128);
conversion_impl!(i32, i8, i64);
conversion_impl!(i32, i16, i64);
conversion_impl!(i32, i64, i128);
conversion_impl!(i64, i8, i128);
conversion_impl!(i64, i16, i128);
conversion_impl!(i64, i32, i128);

// unsigned to signed
conversion_impl!(u8, i8, i16);
conversion_impl!(u8, i16, i32);
conversion_impl!(u8, i32, i64);
conversion_impl!(u8, i64, i128);
conversion_impl!(u16, i8, i32);
conversion_impl!(u16, i16, i32);
conversion_impl!(u16, i32, i64);
conversion_impl!(u16, i64, i128);
conversion_impl!(u32, i8, i64);
conversion_impl!(u32, i16, i64);
conversion_impl!(u32, i32, i64);
conversion_impl!(u32, i64, i128);
conversion_impl!(u64, i8, i128);
conversion_impl!(u64, i16, i128);
conversion_impl!(u64, i32, i128);
conversion_impl!(u64, i64, i128);

// signed to unsigned
conversion_impl!(i8, u8, i16);
conversion_impl!(i8, u16, i32);
conversion_impl!(i8, u32, i64);
conversion_impl!(i8, u64, i128);
conversion_impl!(i16, u8, i32);
conversion_impl!(i16, u16, i32);
conversion_impl!(i16, u32, i64);
conversion_impl!(i16, u64, i128);
conversion_impl!(i32, u8, i64);
conversion_impl!(i32, u16, i64);
conversion_impl!(i32, u32, i64);
conversion_impl!(i32, u64, i128);
conversion_impl!(i64, u8, i128);
conversion_impl!(i64, u16, i128);
conversion_impl!(i64, u32, i128);
conversion_impl!(i64, u64, i128);

// integer to float
conversion_impl!(i8, f32, f32);
conversion_impl!(i8, f64, f64);
conversion_impl!(i16, f32, f32);
conversion_impl!(i16, f64, f64);
conversion_impl!(i32, f32, f32);
conversion_impl!(i32, f64, f64);
conversion_impl!(i64, f32, f32);
conversion_impl!(i64, f64, f64);
conversion_impl!(u8, f32, f32);
conversion_impl!(u8, f64, f64);
conversion_impl!(u16, f32, f32);
conversion_impl!(u16, f64, f64);
conversion_impl!(u32, f32, f32);
conversion_impl!(u32, f64, f64);
conversion_impl!(u64, f32, f32);
conversion_impl!(u64, f64, f64);

// float to integer
conversion_impl!(f32, u8, f32);
conversion_impl!(f32, u16, f32);
conversion_impl!(f32, u32, f32);
conversion_impl!(f32, u64, f32);
conversion_impl!(f32, i8, f32);
conversion_impl!(f32, i16, f32);
conversion_impl!(f32, i32, f32);
conversion_impl!(f32, i64, f32);
conversion_impl!(f64, u8, f64);
conversion_impl!(f64, u16, f64);
conversion_impl!(f64, u32, f64);
conversion_impl!(f64, u64, f64);
conversion_impl!(f64, i8, f64);
conversion_impl!(f64, i16, f64);
conversion_impl!(f64, i32, f64);
conversion_impl!(f64, i64, f64);

// float to float
conversion_impl!(f32, f64, f64);
conversion_impl!(f64, f32, f64);
