//! Multiemu Runtime
//!
//! The main runtime for the multiemu emulator framework

use std::{
    any::Any,
    cmp::Reverse,
    collections::{HashMap, HashSet},
    fmt::Debug,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use nohash::BuildNoHashHasher;
use num::rational::Ratio;
use rustc_hash::FxBuildHasher;
use serde::{Serialize, de::DeserializeOwned};

use crate::{
    component::{Component, ComponentHandle, TypedComponentHandle},
    input::VirtualGamepad,
    machine::{builder::MachineBuilder, registry::ComponentRegistry},
    memory::{AddressSpace, AddressSpaceId, MemoryRemappingCommand},
    path::MultiemuPath,
    persistence::{SaveManager, SnapshotManager},
    platform::{Platform, TestPlatform},
    program::{ProgramManager, ProgramSpecification},
    scheduler::{EventType, Frequency, Period, QueuedEvent, Scheduler},
};

/// Machine builder
pub mod builder;
/// Graphics utilities
pub mod graphics;
pub mod registry;

/// A assembled machine, usable for a further runtime to assist emulation
#[derive(Debug)]
pub struct Machine
where
    Self: Send + Sync,
{
    pub(crate) scheduler: Scheduler,
    /// Memory translation table
    pub address_spaces:
        HashMap<AddressSpaceId, Arc<AddressSpace>, BuildNoHashHasher<AddressSpaceId>>,
    /// All virtual gamepads inserted by components
    pub virtual_gamepads: HashMap<MultiemuPath, Arc<VirtualGamepad>, FxBuildHasher>,
    /// Component Registry
    pub(crate) registry: ComponentRegistry,
    /// All displays this machine has
    pub displays: HashSet<MultiemuPath>,
    /// All audio outputs this machine has
    pub audio_outputs: HashSet<MultiemuPath>,
    /// The program that this machine was set up with, if any
    pub program_specification: Option<ProgramSpecification>,
    #[allow(unused)]
    save_manager: SaveManager,
    #[allow(unused)]
    snapshot_manager: SnapshotManager,
}

impl Machine {
    pub fn build<P: Platform>(
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

    #[inline]
    pub fn address_spaces(&self, address_space_id: AddressSpaceId) -> Option<&Arc<AddressSpace>> {
        self.address_spaces.get(&address_space_id)
    }

    pub fn remap_address_space(
        &self,
        address_space_id: AddressSpaceId,
        commands: impl IntoIterator<Item = MemoryRemappingCommand>,
    ) {
        let address_space = &self.address_spaces[&address_space_id];
        address_space.remap(commands, &self.registry);
    }

    pub fn schedule_event<C: Component>(
        &self,
        time: Period,
        path: &MultiemuPath,
        callback: impl FnOnce(&mut C, Period) + Send + Sync + 'static,
    ) {
        let component = self.registry.handle(path).unwrap();

        self.scheduler.event_queue.queue(QueuedEvent {
            component,
            ty: EventType::Once {
                callback: Box::new(move |component, timestamp| {
                    let component = (component as &mut dyn Any).downcast_mut().unwrap();

                    callback(component, timestamp);
                }),
            },
            time: Reverse(time),
        });
    }

    pub fn schedule_repeating_event<C: Component>(
        &self,
        time: Period,
        frequency: Frequency,
        path: &MultiemuPath,
        mut callback: impl FnMut(&mut C, Period) + Send + Sync + 'static,
    ) {
        let component = self.registry.handle(path).unwrap();

        self.scheduler.event_queue.queue(QueuedEvent {
            component,
            ty: EventType::Repeating {
                callback: Box::new(move |component, timestamp| {
                    let component = (component as &mut dyn Any).downcast_mut().unwrap();

                    callback(component, timestamp);
                }),
                frequency,
            },
            time: Reverse(time),
        });
    }

    pub fn run_duration(&self, allocated_time: Duration) {
        let allocated_time = Period::from_num(allocated_time.as_secs_f32());
        self.scheduler.run(allocated_time);
    }

    pub fn run(&self, allocated_time: Period) {
        self.scheduler.run(allocated_time);
    }

    pub fn now(&self) -> Period {
        self.scheduler.now()
    }

    // Shadow these registry operations so that we can implement them in a way
    // that forces driver components forward
    //
    // So we don't have a critical temporal issue of a non driver component being ahead of a driver component

    pub fn interact<C: Component, T>(
        &self,
        path: &MultiemuPath,
        callback: impl FnOnce(&C) -> T,
    ) -> Option<T> {
        let now = self.now();
        self.scheduler.update_driver_components(now);

        self.registry.interact(path, now, callback)
    }

    pub fn interact_mut<C: Component, T: 'static>(
        &self,
        path: &MultiemuPath,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Option<T> {
        let now = self.now();
        self.scheduler.update_driver_components(now);

        self.registry.interact_mut(path, now, callback)
    }

    pub fn interact_dyn<T>(
        &self,
        path: &MultiemuPath,
        callback: impl FnOnce(&dyn Component) -> T,
    ) -> Option<T> {
        let now = self.now();
        self.scheduler.update_driver_components(now);

        self.registry.interact_dyn(path, now, callback)
    }

    pub fn interact_dyn_mut<T>(
        &self,
        path: &MultiemuPath,
        callback: impl FnOnce(&mut dyn Component) -> T,
    ) -> Option<T> {
        let now = self.now();
        self.scheduler.update_driver_components(now);

        self.registry.interact_dyn_mut(path, now, callback)
    }

    pub fn typed_handle<C: Component>(
        &self,
        path: &MultiemuPath,
    ) -> Option<TypedComponentHandle<C>> {
        self.registry.typed_handle(path)
    }

    pub fn component_handle(&self, path: &MultiemuPath) -> Option<ComponentHandle> {
        self.registry.handle(path)
    }
}

/// Helper trait representing a fully constructed machine
pub trait MachineFactory<P: Platform>: Send + Sync + 'static {
    /// Construct a new machine given the parameters
    fn construct(&self, machine_builder: MachineBuilder<P>) -> MachineBuilder<P>;
}

pub trait Quirks: Serialize + DeserializeOwned + Debug + Clone + Default + 'static {}
impl<T: Serialize + DeserializeOwned + Debug + Clone + Default + 'static> Quirks for T {}
