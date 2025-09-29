//! Multiemu Runtime
//!
//! The main runtime for the multiemu emulator framework

use crate::{
    audio::{AudioOutputId},
    builder::MachineBuilder,
    component::ComponentRegistry,
    graphics::FramebufferStorage,
    memory::MemoryAccessTable,
    platform::{Platform, TestPlatform},
    save::{SaveManager, SnapshotManager},
    utils::DirectMainThreadExecutor,
};
use input::{VirtualGamepad, VirtualGamepadId};
use multiemu_rom::{ROM_INFORMATION_TABLE, RomId, RomInfo, RomMetadata, System};
use num::rational::Ratio;
use redb::ReadableDatabase;
use rustc_hash::FxBuildHasher;
use scheduler::Scheduler;
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
    path::PathBuf,
    sync::{Arc, Mutex},
};

/// Audio related types
pub mod audio;
/// Machine builder
pub mod builder;
/// Component related types
pub mod component;
/// Graphics utilities
pub mod graphics;
/// Input related types
pub mod input;
/// Memory related types
pub mod memory;
/// Platform abstraction traits
pub mod platform;
/// Barebones processor related types
pub mod processor;
/// Save related types
mod save;
/// The scheduler
pub mod scheduler;
/// Misc utilities
pub mod utils;

pub use save::Slot as SnapshotSlot;

/// A assembled machine, usable for a further runtime to assist emulation
///
/// Note: This should all be interior mutable
///
/// This should only be dropped on the main thread. Dropping it outside the main thread may result in a abort or a panic, but not UB
#[derive(Debug)]
pub struct Machine<P: Platform>
where
    Self: Send + Sync,
{
    /// Scheduler loaded with tasks
    pub scheduler: Mutex<Scheduler>,
    /// Memory translation table
    pub memory_access_table: Arc<MemoryAccessTable>,
    /// All virtual gamepads inserted by components
    pub virtual_gamepads: HashMap<VirtualGamepadId, Arc<VirtualGamepad>, FxBuildHasher>,
    /// The store to interact with components
    pub component_registry: Arc<ComponentRegistry>,
    /// All displays this machine has
    pub graphics_manager: Arc<FramebufferStorage<P::GraphicsApi>>,
    pub user_specified_roms: Option<UserSpecifiedRoms>,
    save_manager: SaveManager,
    snapshot_manager: SnapshotManager,
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
        Self::build(
            None,
            Arc::new(RomMetadata::new(None, None).unwrap()),
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
