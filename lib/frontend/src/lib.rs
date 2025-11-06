mod backend;
pub mod environment;
mod frontend;
mod gui;
mod hotkey;
mod machine_factories;
mod platform;

pub use backend::*;
pub use frontend::*;
pub use gui::software_rendering as gui_software_rendering;
pub use hotkey::*;
pub use machine_factories::MachineFactories;
pub use platform::*;

/// Canonical shader for egui rendering
pub const EGUI_WGSL_SHADER: &str = include_str!("egui.wgsl");
