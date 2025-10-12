//! Multiemu Runtime
//!
//! The main runtime for the multiemu emulator framework

use crate::{
    component::ResourcePath,
    environment::{ENVIRONMENT_LOCATION, Environment},
    machine::{
        builder::MachineBuilder, registry::ComponentRegistry, virtual_gamepad::VirtualGamepad,
    },
    memory::MemoryAccessTable,
    persistence::{SaveManager, SnapshotManager},
    platform::{Platform, TestPlatform},
    program::{ProgramMetadata, ProgramSpecification},
    scheduler::{SchedulerHandle, SchedulerState},
    utils::DirectMainThreadExecutor,
};
use num::rational::Ratio;
use rustc_hash::FxBuildHasher;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    fs::File,
    marker::PhantomData,
    ops::Deref,
    path::PathBuf,
    sync::{Arc, RwLock},
};

/// Machine builder
pub mod builder;
/// Graphics utilities
pub mod graphics;
pub mod registry;
pub mod virtual_gamepad;

/// A assembled machine, usable for a further runtime to assist emulation
///
/// This should only be dropped on the main thread. Dropping it outside the main thread may result in a abort or a panic, but not UB
#[derive(Debug)]
pub struct Machine<P: Platform>
where
    Self: Send + Sync,
{
    /// Scheduler controller
    pub scheduler_handle: Arc<SchedulerHandle>,
    // A dedicated thread might own the actual scheduler state, if this is present you need to drive it
    pub scheduler_state: Option<SchedulerState>,
    /// Memory translation table
    pub memory_access_table: Arc<MemoryAccessTable>,
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
    save_manager: SaveManager,
    snapshot_manager: SnapshotManager,
    _phantom: PhantomData<fn() -> P>,
}

impl Machine<TestPlatform> {
    pub fn build_test(
        program_specification: Option<ProgramSpecification>,
        program_manager: Arc<ProgramMetadata>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
    ) -> MachineBuilder<TestPlatform> {
        Self::build(
            program_specification,
            program_manager,
            save_path,
            snapshot_path,
            Ratio::from_integer(44100),
            Arc::new(DirectMainThreadExecutor),
        )
    }

    pub fn build_test_minimal() -> MachineBuilder<TestPlatform> {
        let environment_file = File::create(ENVIRONMENT_LOCATION.deref()).unwrap();
        let environment: Environment = ron::de::from_reader(environment_file).unwrap_or_default();

        let environment = Arc::new(RwLock::new(environment));

        Self::build(
            None,
            Arc::new(ProgramMetadata::new_test(environment).unwrap()),
            None,
            None,
            Ratio::from_integer(44100),
            Arc::new(DirectMainThreadExecutor),
        )
    }
}

impl<P: Platform> Machine<P> {
    pub fn build(
        program_specification: Option<ProgramSpecification>,
        program_manager: Arc<ProgramMetadata>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
        sample_rate: Ratio<u32>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> MachineBuilder<P> {
        MachineBuilder::new(
            program_specification,
            program_manager,
            save_path,
            snapshot_path,
            sample_rate,
            main_thread_executor,
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
