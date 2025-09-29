use crate::{
    GraphicsRuntime,
    backend::{AudioContext, DisplayApiHandle},
    machine_factories::MachineFactories,
};
use egui::RawInput;
use multiemu_config::Environment;
use multiemu_rom::RomMetadata;
use multiemu_runtime::{UserSpecifiedRoms, platform::Platform};
use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

pub trait PlatformExt: Platform + Sized + 'static {
    type AudioRuntime: AudioContext<Self>;
    type GraphicsRuntime: GraphicsRuntime<Self>;
    type EguiPlatformIntegration: EguiPlatformIntegration<
        <Self::GraphicsRuntime as GraphicsRuntime<Self>>::DisplayApiHandle,
    >;

    fn run(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomMetadata>,
        machine_factories: MachineFactories<Self>,
    ) -> Result<(), Box<dyn std::error::Error>>;

    fn run_with_machine(
        environment: Arc<RwLock<Environment>>,
        rom_manager: Arc<RomMetadata>,
        machine_factories: MachineFactories<Self>,
        user_specified_roms: UserSpecifiedRoms,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

pub trait EguiPlatformIntegration<D: DisplayApiHandle>: Debug + 'static {
    fn set_egui_context(&mut self, context: &egui::Context);
    fn gather_platform_specific_inputs(&mut self) -> RawInput;
}
