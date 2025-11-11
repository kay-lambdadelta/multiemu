//! Multiemu Runtime
//!
//! The main runtime for the multiemu emulator framework

use crate::{
    component::ResourcePath,
    input::VirtualGamepad,
    machine::{builder::MachineBuilder, registry::ComponentRegistry},
    memory::{AddressSpace, AddressSpaceId},
    persistence::{SaveManager, SnapshotManager},
    platform::{Platform, TestPlatform},
    program::{ProgramManager, ProgramSpecification},
    scheduler::Scheduler,
};
use num::rational::Ratio;
use rustc_hash::FxBuildHasher;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    marker::PhantomData,
    path::PathBuf,
    sync::Arc,
};

/// Machine builder
pub mod builder;
/// Graphics utilities
pub mod graphics;
pub mod registry;

/// A assembled machine, usable for a further runtime to assist emulation
///
/// This should only be dropped on the main thread. Dropping it outside the main thread may result in a abort or a panic, but not UB
#[derive(Debug)]
pub struct Machine<P: Platform>
where
    Self: Send + Sync,
{
    // A dedicated thread might own the actual scheduler state, if this is present you need to drive it
    pub scheduler: Option<Scheduler>,
    /// Memory translation table
    pub address_spaces: HashMap<AddressSpaceId, Arc<AddressSpace>>,
    /// All virtual gamepads inserted by components
    pub virtual_gamepads: HashMap<ResourcePath, Arc<VirtualGamepad>, FxBuildHasher>,
    /// The store to interact with components
    pub component_registry: Arc<ComponentRegistry>,
    /// All displays this machine has
    pub displays: HashSet<ResourcePath>,
    /// All audio outputs this machine has
    pub audio_outputs: HashSet<ResourcePath>,
    /// The program that this machine was set up with, if any
    pub program_specification: Option<ProgramSpecification>,
    #[allow(unused)]
    save_manager: SaveManager,
    #[allow(unused)]
    snapshot_manager: SnapshotManager,
    _phantom: PhantomData<fn() -> P>,
}

impl Machine<TestPlatform> {
    pub fn build_test(
        program_specification: Option<ProgramSpecification>,
        program_manager: Arc<ProgramManager>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
    ) -> MachineBuilder<TestPlatform> {
        Self::build(
            program_specification,
            program_manager,
            save_path,
            snapshot_path,
            Ratio::from_integer(44100),
        )
    }

    pub fn build_test_minimal() -> MachineBuilder<TestPlatform> {
        Self::build(
            None,
            Arc::new(ProgramManager::default()),
            None,
            None,
            Ratio::from_integer(44100),
        )
    }
}

impl<P: Platform> Machine<P> {
    pub fn build(
        program_specification: Option<ProgramSpecification>,
        program_manager: Arc<ProgramManager>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
        sample_rate: Ratio<u32>,
    ) -> MachineBuilder<P> {
        MachineBuilder::new(
            program_specification,
            program_manager,
            save_path,
            snapshot_path,
            sample_rate,
        )
    }
}

/// Helper trait representing a fully constructed machine
pub trait MachineFactory<P: Platform>: Send + Sync + 'static {
    /// Construct a new machine given the parameters
    fn construct(&self, machine_builder: MachineBuilder<P>) -> MachineBuilder<P>;
}

/// Implement for closures
impl<P: Platform, F: Fn(MachineBuilder<P>) -> MachineBuilder<P> + Send + Sync + 'static>
    MachineFactory<P> for F
{
    fn construct(&self, machine_builder: MachineBuilder<P>) -> MachineBuilder<P> {
        self(machine_builder)
    }
}
