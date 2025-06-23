mod backend;
mod gui;
mod machine_factories;
mod platform;
mod runtime;

pub use backend::*;
pub use gui::software_rendering as gui_software_rendering;
pub use machine_factories::MachineFactories;
pub use platform::*;
pub use runtime::*;

pub const EGUI_WGSL_SHADER: &'static str = include_str!("../shaders/egui.wgsl");
