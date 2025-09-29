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
pub use info::RomInfo;
pub use manager::{RomMetadata, *};
pub use system::{AtariSystem, NintendoSystem, OtherSystem, SegaSystem, SonySystem, System};
