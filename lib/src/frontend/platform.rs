use crate::{
    environment::Environment,
    frontend::{AudioRuntime, DisplayApiHandle, GraphicsRuntime, MachineFactories},
    machine::UserSpecifiedRoms,
    platform::Platform,
    rom::RomMetadata,
};
use egui::RawInput;
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

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
        rom_manager: Arc<RomMetadata>,
        machine_factories: MachineFactories<Self>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Run launching a machine
    fn run_with_machine(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomMetadata>,
        machine_factories: MachineFactories<Self>,
        user_specified_roms: UserSpecifiedRoms,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// Glue type between the frontend and extra stuff egui might need
pub trait EguiPlatformIntegration<D: DisplayApiHandle>: Debug + 'static {
    /// Set new egui context
    fn set_egui_context(&mut self, context: &egui::Context);
    /// Gather inputs that can't be represented in [multiemu]'s input system
    fn gather_platform_specific_inputs(&mut self) -> RawInput;
}
