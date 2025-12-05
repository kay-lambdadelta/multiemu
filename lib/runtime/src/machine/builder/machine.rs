use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet},
    marker::PhantomData,
    ops::RangeInclusive,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use indexmap::IndexMap;
use num::rational::Ratio;
use rustc_hash::FxBuildHasher;

use crate::{
    component::{Component, ComponentConfig, ComponentPath, LateInitializedData},
    graphics::GraphicsApi,
    machine::{
        Machine,
        builder::{
            AddressSpaceInfo, ComponentBuilder, ComponentMetadata, PartialEvent,
            SchedulerParticipation,
        },
        graphics::GraphicsRequirements,
        registry::ComponentRegistry,
    },
    memory::{
        Address, AddressSpace, AddressSpaceId, MapTarget, MemoryRemappingCommand, Permissions,
    },
    persistence::{SaveManager, SnapshotManager},
    platform::Platform,
    program::{MachineId, ProgramManager, ProgramSpecification},
    scheduler::{QueuedEvent, Scheduler},
};

/// Builder to produce a machine, definition crates will want to use this
pub struct MachineBuilder<P: Platform> {
    /// Rom manager
    pub(super) program_manager: Arc<ProgramManager>,
    /// Save manager
    pub(super) save_manager: SaveManager,
    /// Snapshot manager
    pub(super) snapshot_manager: SnapshotManager,
    /// Selected sample rate
    pub(super) sample_rate: Ratio<u32>,
    /// The store for components
    pub(super) registry: ComponentRegistry,
    /// Component metadata
    pub(super) component_metadata: IndexMap<ComponentPath, ComponentMetadata<P>, FxBuildHasher>,
    /// Program we were opened with
    pub(super) program_specification: Option<ProgramSpecification>,
    /// Scheduler
    pub(super) scheduler: Scheduler,
    pub(super) address_spaces: HashMap<AddressSpaceId, AddressSpaceInfo>,
    pub(super) next_address_space_id: AddressSpaceId,
}

impl<P: Platform> MachineBuilder<P> {
    pub(crate) fn new(
        program_specification: Option<ProgramSpecification>,
        program_manager: Arc<ProgramManager>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
        sample_rate: Ratio<u32>,
    ) -> Self {
        let scheduler = Scheduler::new();

        let registry = ComponentRegistry::default();
        let save_manager = SaveManager::new(save_path);
        let snapshot_manager = SnapshotManager::new(snapshot_path);

        MachineBuilder::<P> {
            address_spaces: HashMap::default(),
            save_manager,
            snapshot_manager,
            program_manager,
            sample_rate,
            component_metadata: IndexMap::default(),
            program_specification,
            scheduler,
            registry,
            next_address_space_id: AddressSpaceId(0),
        }
    }

    pub fn machine_id(&self) -> Option<MachineId> {
        self.program_specification
            .as_ref()
            .map(|program_specification| program_specification.id.machine)
    }

    pub fn program_specification(&self) -> Option<&ProgramSpecification> {
        self.program_specification.as_ref()
    }

    pub fn program_manager(&self) -> &Arc<ProgramManager> {
        &self.program_manager
    }

    #[inline]
    fn insert_component_with_path<B: ComponentConfig<P>>(
        &mut self,
        path: ComponentPath,
        config: B,
    ) {
        let mut component_metadata = ComponentMetadata::new::<B>();

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: self,
            component_metadata: &mut component_metadata,
            path: &path,
            _phantom: PhantomData,
        };

        let component = config
            .build_component(component_builder)
            .expect("Failed to build component");

        self.registry.insert_component(
            path.clone(),
            component_metadata.scheduler_participation,
            self.scheduler.event_queue.clone(),
            component,
        );

        self.component_metadata.insert(path, component_metadata);
    }

    /// Insert a component into the machine
    #[inline]
    pub fn insert_component<B: ComponentConfig<P>>(
        mut self,
        name: &str,
        config: B,
    ) -> (Self, ComponentPath) {
        assert!(
            !name.contains(ComponentPath::SEPARATOR),
            "This function requires a name not a path"
        );

        let path = ComponentPath::from_str(name).unwrap();
        self.insert_component_with_path(path.clone(), config);

        (self, path)
    }

    /// Insert a component with a default config
    #[inline]
    pub fn insert_default_component<B: ComponentConfig<P> + Default>(
        self,
        name: &str,
    ) -> (Self, ComponentPath) {
        let config = B::default();
        self.insert_component(name, config)
    }

    /// Insert the required information to construct a address space
    pub fn insert_address_space(mut self, address_space_width: u8) -> (Self, AddressSpaceId) {
        assert!(
            (address_space_width <= usize::BITS as u8),
            "This host machine cannot handle an address space of {address_space_width} bits"
        );

        let address_space = Arc::new(AddressSpace::new(
            self.next_address_space_id,
            address_space_width,
        ));
        let address_space_id = address_space.id();
        self.next_address_space_id.0 = self
            .next_address_space_id
            .0
            .checked_add(1)
            .expect("Too many address spaces");
        self.address_spaces.insert(
            address_space.id(),
            AddressSpaceInfo {
                address_space,
                memory_map_queue: Vec::default(),
            },
        );

        (self, address_space_id)
    }

    pub fn memory_map_mirror_read(
        mut self,
        address_space: AddressSpaceId,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
    ) -> Self {
        self.address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range: source,
                target: MapTarget::Mirror { destination },
                permissions: Permissions {
                    read: true,
                    write: false,
                },
            });

        self
    }

    pub fn memory_map_mirror_write(
        mut self,
        address_space: AddressSpaceId,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
    ) -> Self {
        self.address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range: source,
                target: MapTarget::Mirror { destination },
                permissions: Permissions {
                    read: false,
                    write: true,
                },
            });

        self
    }

    pub fn memory_map_mirror(
        mut self,
        address_space: AddressSpaceId,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
    ) -> Self {
        self.address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range: source,
                target: MapTarget::Mirror { destination },
                permissions: Permissions {
                    read: true,
                    write: true,
                },
            });

        self
    }

    pub fn interact<C: Component, T>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&C) -> T,
    ) -> Option<T> {
        self.registry
            .interact_without_synchronization(path, callback)
    }

    pub fn interact_mut<C: Component, T: 'static>(
        &self,
        path: &ComponentPath,
        callback: impl FnOnce(&mut C) -> T,
    ) -> Option<T> {
        self.registry
            .interact_mut_without_synchronization(path, callback)
    }

    pub fn graphics_requirements(&self) -> GraphicsRequirements<P::GraphicsApi> {
        self.component_metadata
            .values()
            .map(|metadata| &metadata.graphics_requirements)
            .fold(GraphicsRequirements::default(), |acc, value| {
                acc | value.clone()
            })
    }

    /// Build the machine
    pub fn build(
        mut self,
        component_graphics_initialization_data: <P::GraphicsApi as GraphicsApi>::InitializationData,
    ) -> Arc<Machine> {
        let mut virtual_gamepads = HashMap::default();
        let mut audio_outputs = HashSet::new();
        let mut component_initializers = HashMap::new();
        let mut displays = HashSet::default();

        for (path, component_metadata) in self.component_metadata.drain(..) {
            let component_handle = self.registry.handle(&path).unwrap();

            component_initializers.insert(path.clone(), component_metadata.late_initializer);
            displays.extend(component_metadata.displays);
            virtual_gamepads.extend(component_metadata.gamepads);
            audio_outputs.extend(component_metadata.audio_outputs);

            if component_metadata.scheduler_participation == SchedulerParticipation::SchedulerDriven
            {
                self.scheduler
                    .register_driven_component(path, component_handle.clone());
            }

            for PartialEvent { ty, time } in component_metadata.events {
                self.scheduler.event_queue.queue(QueuedEvent {
                    component: component_handle.clone(),
                    ty,
                    time: Reverse(time),
                });
            }
        }

        for AddressSpaceInfo {
            address_space,
            memory_map_queue,
        } in self.address_spaces.values_mut()
        {
            address_space.remap(memory_map_queue.drain(..), &self.registry);
        }

        let address_spaces = self
            .address_spaces
            .into_iter()
            .map(|(id, info)| (id, info.address_space))
            .collect();

        let machine = Arc::new(Machine {
            scheduler: self.scheduler,
            address_spaces,
            virtual_gamepads,
            registry: self.registry,
            displays,
            save_manager: self.save_manager,
            snapshot_manager: self.snapshot_manager,
            program_specification: self.program_specification,
            audio_outputs,
        });

        let late_initialized_data = LateInitializedData::<P> {
            machine: Arc::downgrade(&machine),
            component_graphics_initialization_data,
        };

        for (component_path, initializer) in component_initializers {
            machine
                .registry
                .interact_dyn_mut_without_synchronization(&component_path, |component| {
                    initializer(component, &late_initialized_data);
                })
                .unwrap();
        }

        machine
    }
}
