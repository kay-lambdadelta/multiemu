use egui::RawInput;
use multiemu_base::{
    environment::Environment,
    platform::Platform,
    program::{ProgramMetadata, ProgramSpecification},
};
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use crate::{AudioRuntime, DisplayApiHandle, GraphicsRuntime, MachineFactories};

/// Extension trait for the platform relevant to the frontend
pub trait PlatformExt: Platform + Sized + 'static {
    /// Audio runtime
    type AudioRuntime: AudioRuntime<Self>;
    /// Graphics runtime
    type GraphicsRuntime: GraphicsRuntime<Self>;
    /// Glue type between the frontend and extra stuff egui might need
    type EguiPlatformIntegration: EguiPlatformIntegration<
        <Self::GraphicsRuntime as GraphicsRuntime<Self>>::DisplayApiHandle,
    >;

    /// Run
    fn run(
        environment: Arc<RwLock<Environment>>,
        program_manager: Arc<ProgramMetadata>,
        machine_factories: MachineFactories<Self>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Run launching a machine
    fn run_with_program(
        environment: Arc<RwLock<Environment>>,
        program_manager: Arc<ProgramMetadata>,
        machine_factories: MachineFactories<Self>,
        program: ProgramSpecification,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Glue type between the frontend and extra stuff egui might need
pub trait EguiPlatformIntegration<D: DisplayApiHandle>: Debug + 'static {
    /// Set new egui context
    fn set_egui_context(&mut self, context: &egui::Context);
    /// Gather inputs that can't be represented in [multiemu]'s input system
    fn gather_platform_specific_inputs(&mut self) -> RawInput;
}
