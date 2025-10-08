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
    rom::{ROM_INFORMATION_TABLE, RomId, RomInfo, RomMetadata, System},
    scheduler::{SchedulerHandle, SchedulerState},
    utils::DirectMainThreadExecutor,
};
use num::rational::Ratio;
use redb::ReadableDatabase;
use rustc_hash::FxBuildHasher;
use std::{
    borrow::Cow,
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
    pub user_specified_roms: Option<UserSpecifiedRoms>,
    save_manager: SaveManager,
    snapshot_manager: SnapshotManager,
    _phantom: PhantomData<fn() -> P>,
}

impl Machine<TestPlatform> {
    pub fn build_test(
        user_specified_roms: Option<UserSpecifiedRoms>,
        rom_manager: Arc<RomMetadata>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
    ) -> MachineBuilder<TestPlatform> {
        Self::build(
            user_specified_roms,
            rom_manager,
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
            Arc::new(RomMetadata::new_test(environment).unwrap()),
            None,
            None,
            Ratio::from_integer(44100),
            Arc::new(DirectMainThreadExecutor),
        )
    }
}

impl<P: Platform> Machine<P> {
    pub fn build(
        user_specified_roms: Option<UserSpecifiedRoms>,
        rom_manager: Arc<RomMetadata>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
        sample_rate: Ratio<u32>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> MachineBuilder<P> {
        MachineBuilder::new(
            user_specified_roms,
            rom_manager,
            save_path,
            snapshot_path,
            sample_rate,
            main_thread_executor,
        )
    }

    pub fn system(&self) -> Option<System> {
        self.user_specified_roms
            .as_ref()
            .map(|roms| roms.main.identity.system())
    }

    pub fn store_save(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self.user_specified_roms.as_ref() {
            Some(user_specified_roms) => {
                let rom_id = user_specified_roms.main.id;
                let rom_name = user_specified_roms.main.identity.name();

                self.save_manager
                    .write(rom_id, rom_name, &self.component_registry)?;
            }
            None => todo!(),
        }

        Ok(())
    }

    pub fn store_snapshot(&self, slot: u16) -> Result<(), Box<dyn std::error::Error>> {
        match self.user_specified_roms.as_ref() {
            Some(user_specified_roms) => {
                let rom_id = user_specified_roms.main.id;
                let rom_name = user_specified_roms.main.identity.name();

                self.snapshot_manager
                    .write(rom_id, rom_name, slot, &self.component_registry)?;
            }
            None => todo!(),
        }

        Ok(())
    }

    pub fn load_snapshot(&self, slot: u16) -> Result<(), Box<dyn std::error::Error>> {
        match self.user_specified_roms.as_ref() {
            Some(user_specified_roms) => {
                let rom_id = user_specified_roms.main.id;
                let rom_name = user_specified_roms.main.identity.name();

                self.snapshot_manager
                    .read(rom_id, rom_name, slot, &self.component_registry)?;
            }
            None => todo!(),
        }

        Ok(())
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

#[derive(Debug, Clone)]
pub struct RomSpecification {
    pub id: RomId,
    pub identity: RomInfo,
}

#[derive(Debug, Clone)]
pub struct UserSpecifiedRoms {
    /// Identity of the main rom
    pub main: RomSpecification,
    /// Associated subroms
    pub sub: Cow<'static, [RomSpecification]>,
}

impl UserSpecifiedRoms {
    /// TODO: make less naive
    pub fn from_id(
        id: RomId,
        rom_manager: &RomMetadata,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let transaction = rom_manager.rom_information.begin_read()?;
        let table = transaction.open_multimap_table(ROM_INFORMATION_TABLE)?;
        let info = table.get(id)?.next().unwrap()?.value();

        Ok(Self {
            main: RomSpecification { id, identity: info },
            sub: Cow::Borrowed(&[]),
        })
    }
}
