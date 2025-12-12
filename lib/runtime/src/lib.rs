//! Multiemu Runtime
//!
//! Main runtime crate for the multiemu framework

/// Basic types relating to the fundemental unit of this emulator
pub mod component;
/// Graphics definitions
pub mod graphics;
/// Input definitions
pub mod input;
/// Machine builder and definition
pub mod machine;
/// Memory access utilities
pub mod memory;
/// Path
pub mod path;
/// Saves and snapshots
pub mod persistence;
/// Platform description utilities
pub mod platform;
/// Emulated processor utilities
pub mod processor;
/// Types and tools to help identify and describe a emulated program
pub mod program;
/// Emulator scheduler
pub mod scheduler;
#[cfg(any(feature = "vulkan", feature = "opengl"))]
/// WGSL shader compilers
pub mod shader;
