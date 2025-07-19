//! Multiemu Save
//! 
//! A library containing definitions and managers for the multiemu save and snapshot system

mod component_name;
mod manager;

pub use component_name::*;
pub use manager::*;

/// Version that components use
pub type ComponentVersion = u32;
