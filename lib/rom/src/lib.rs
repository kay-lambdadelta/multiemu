//! Multiemu Rom
//!
//! A helper library for operating on rom files

#![deny(missing_docs)]

mod graphics;
mod id;
mod info;
mod manager;
mod system;

pub use id::RomId;
pub use info::RomInfoV0 as RomInfo;
pub use manager::{RomManager, *};
pub use system::{AtariSystem, GameSystem, NintendoSystem, OtherSystem, SegaSystem, SonySystem};
