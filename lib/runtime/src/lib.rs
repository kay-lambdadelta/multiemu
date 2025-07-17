//! Multiemu Runtime
//!
//! The main runtime for the multiemu emulator framework

use crate::{
    audio::{AudioOutputId, AudioOutputInfo},
    builder::MachineBuilder,
    component::ComponentRegistry,
    graphics::{DisplayId, DisplayInfo},
    memory::MemoryAccessTable,
    platform::Platform,
};
use input::{VirtualGamepad, VirtualGamepadId};
use multiemu_rom::{RomId, RomManager};
use multiemu_save::{SaveManager, SnapshotManager};
use rustc_hash::FxBuildHasher;
use scheduler::Scheduler;
use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Debug,
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
/// The scheduler
pub mod scheduler;
/// Misc utilities
pub mod utils;

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
    pub displays: HashMap<DisplayId, DisplayInfo<P::GraphicsApi>, FxBuildHasher>,
    /// All audio outputs this machine has
    pub audio_outputs: HashMap<AudioOutputId, AudioOutputInfo<P::SampleFormat>, FxBuildHasher>,
    pub rom_manager: Arc<RomManager>,
    pub save_manager: Arc<SaveManager>,
    pub snapshot_manager: Arc<SnapshotManager>,
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
pub struct UserSpecifiedRoms {
    /// The ROM the machine will base identification on
    pub main: RomId,
    /// Associated subroms
    pub sub: Cow<'static, [RomId]>,
}
