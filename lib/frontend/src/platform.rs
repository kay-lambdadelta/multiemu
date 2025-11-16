use std::{fmt::Debug, sync::Arc};

use egui::RawInput;
use multiemu_runtime::{
    platform::Platform,
    program::{ProgramManager, ProgramSpecification},
};

use crate::{
    AudioRuntime, GraphicsRuntime, MachineFactories, WindowingHandle, environment::Environment,
};

/// Extension trait for the platform relevant to the frontend
pub trait PlatformExt: Platform + Sized + 'static {
    /// Audio runtime
    type AudioRuntime: AudioRuntime<Self>;
    /// Graphics runtime
    type GraphicsRuntime: GraphicsRuntime<Self>;
    /// Glue type between the frontend and extra stuff egui might need
    type EguiWindowingIntegration: EguiWindowingIntegration<
        <Self::GraphicsRuntime as GraphicsRuntime<Self>>::WindowingHandle,
    >;

    /// Run
    fn run(
        environment: Environment,
        program_manager: Arc<ProgramManager>,
        machine_factories: MachineFactories<Self>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Run launching a machine
    fn run_with_program(
        environment: Environment,
        program_manager: Arc<ProgramManager>,
        machine_factories: MachineFactories<Self>,
        program: ProgramSpecification,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Glue type between the frontend and extra stuff egui might need
pub trait EguiWindowingIntegration<D: WindowingHandle>: Debug + 'static {
    /// Set new egui context
    fn set_egui_context(&mut self, context: &egui::Context);
    /// Gather inputs that can't be represented in [multiemu]'s input system
    fn gather_platform_specific_inputs(&mut self) -> RawInput;
}
