#![no_std]

//! Audio utilities

mod frame;
mod generation;
mod interpolate;
mod sample;

pub use frame::FrameIterator;
pub use generation::*;
pub use interpolate::*;
pub use sample::*;
