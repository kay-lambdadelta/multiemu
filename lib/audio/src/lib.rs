//! A basic yet featureful audio library for the multiemu emulator framework
//!
//! This library was created due to a (in my view) lack of good general purpose audio libraries in rust

#![no_std]
#![deny(missing_docs)]

mod frame;
mod generation;
mod interpolate;
mod sample;

pub use frame::FrameIterator;
pub use generation::*;
pub use interpolate::*;
pub use sample::*;
