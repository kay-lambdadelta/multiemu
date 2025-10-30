use crate::{
    component::{
        Component, ComponentConfig, ComponentPath, ComponentVersion, LateInitializedData,
        ResourcePath,
    },
    graphics::GraphicsApi,
    machine::{
        Machine, graphics::GraphicsRequirements, registry::ComponentRegistry,
        virtual_gamepad::VirtualGamepad,
    },
    memory::{Address, AddressSpaceId, MemoryAccessTable, MemoryRemappingCommand, Permissions},
    persistence::{SaveManager, SnapshotManager},
    platform::Platform,
    program::{MachineId, ProgramManager, ProgramSpecification},
    scheduler::{Scheduler, Task, TaskData, TaskId, TaskType},
};
use indexmap::IndexMap;
use num::rational::Ratio;
use rustc_hash::FxBuildHasher;
use std::{
    any::Any,
    borrow::Cow,
    collections::{HashMap, HashSet},
    io::Read,
    marker::PhantomData,
    ops::RangeInclusive,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

/// Overall data extracted from components needed for machine initialization
pub struct ComponentMetadata<P: Platform> {
    pub displays: HashSet<ResourcePath>,
    pub graphics_requirements: GraphicsRequirements<P::GraphicsApi>,
    pub audio_outputs: HashSet<ResourcePath>,
    pub gamepads: HashMap<ResourcePath, Arc<VirtualGamepad>>,
    #[allow(clippy::type_complexity)]
    pub late_initializer: Option<Box<dyn FnOnce(&mut dyn Component, &LateInitializedData<P>)>>,
}

impl<P: Platform> Default for ComponentMetadata<P> {
    fn default() -> Self {
        Self {
            displays: Default::default(),
            graphics_requirements: Default::default(),
            audio_outputs: Default::default(),
            gamepads: Default::default(),
            late_initializer: None,
        }
    }
}

/// Builder to produce a machine, definition crates will want to use this
pub struct MachineBuilder<P: Platform> {
    /// Memory translation table
    memory_access_table: Arc<MemoryAccessTable>,
    /// Rom manager
    rom_metadata: Arc<ProgramManager>,
    /// Save manager
    save_manager: SaveManager,
    /// Snapshot manager
    snapshot_manager: SnapshotManager,
    /// Selected sample rate
    sample_rate: Ratio<u32>,
    /// The store for components
    registry: Arc<ComponentRegistry>,
    /// Component metadata
    component_metadata: IndexMap<ComponentPath, ComponentMetadata<P>, FxBuildHasher>,
    /// Program we were opened with
    program_specification: Option<ProgramSpecification>,
    /// Scheduler
    scheduler: Scheduler,
}

impl<P: Platform> MachineBuilder<P> {
    pub(crate) fn new(
        program_specification: Option<ProgramSpecification>,
        program_manager: Arc<ProgramManager>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
        sample_rate: Ratio<u32>,
    ) -> Self {
        let registry = Arc::new(ComponentRegistry::default());
        let save_manager = SaveManager::new(save_path);
        let snapshot_manager = SnapshotManager::new(snapshot_path);
        let scheduler = Scheduler::new(registry.clone());

        MachineBuilder::<P> {
            memory_access_table: Arc::new(MemoryAccessTable::new(registry.clone())),
            save_manager,
            snapshot_manager,
            rom_metadata: program_manager,
            sample_rate,
            component_metadata: IndexMap::default(),
            program_specification,
            scheduler,
            registry,
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
        &self.rom_metadata
    }

    pub fn registry(&self) -> &ComponentRegistry {
        &self.registry
    }

    #[inline]
    fn insert_component_with_path<B: ComponentConfig<P>>(
        &mut self,
        path: ComponentPath,
        config: B,
    ) {
        let mut tasks = HashMap::new();
        let mut component_metadata = ComponentMetadata::default();

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: self,
            component_metadata: &mut component_metadata,
            tasks: &mut tasks,
            path: &path,
            _phantom: PhantomData,
        };

        let component = config
            .build_component(component_builder)
            .expect("Failed to build component");

        self.registry
            .insert_component(path.clone(), component, tasks);

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
    pub fn insert_address_space(mut self, width: u8) -> (Self, AddressSpaceId) {
        assert!(
            (width <= usize::BITS as u8),
            "This host machine cannot handle an address space of {width} bits"
        );

        let mutable_access_table = Arc::get_mut(&mut self.memory_access_table)
            .expect("Address spaces must be added before memory access table is spread");

        let id = mutable_access_table.insert_address_space(width);

        (self, id)
    }

    pub fn memory_map_mirror_read(
        self,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
        address_space: AddressSpaceId,
    ) -> Self {
        self.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Mirror {
                source,
                destination,
                permissions: Permissions {
                    read: true,
                    write: false,
                },
            }],
        );

        self
    }

    pub fn memory_map_mirror_write(
        self,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
        address_space: AddressSpaceId,
    ) -> Self {
        self.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Mirror {
                source,
                destination,
                permissions: Permissions {
                    read: false,
                    write: true,
                },
            }],
        );

        self
    }

    pub fn memory_map_mirror(
        self,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
        address_space: AddressSpaceId,
    ) -> Self {
        self.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Mirror {
                source,
                destination,
                permissions: Permissions {
                    read: true,
                    write: true,
                },
            }],
        );

        self
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
    ) -> Machine<P> {
        let mut virtual_gamepads = HashMap::default();
        let mut audio_outputs = HashSet::new();
        let mut component_initializers = HashMap::new();
        let mut displays = HashSet::default();

        for (path, component_metadata) in self.component_metadata.drain(..) {
            if let Some(initializer) = component_metadata.late_initializer {
                component_initializers.insert(path, initializer);
            }

            displays.extend(component_metadata.displays);
            virtual_gamepads.extend(component_metadata.gamepads);
            audio_outputs.extend(component_metadata.audio_outputs);
        }

        let late_initialized_data = LateInitializedData::<P> {
            component_graphics_initialization_data,
        };

        for (component_path, initializer) in component_initializers {
            self.registry
                .interact_dyn_mut(&component_path, |component| {
                    initializer(component, &late_initialized_data);
                })
                .unwrap();
        }

        // Build the timeline before building the machine
        self.scheduler.build_timeline();

        Machine {
            scheduler: Some(self.scheduler),
            memory_access_table: self.memory_access_table.clone(),
            virtual_gamepads,
            component_registry: self.registry,
            displays,
            save_manager: self.save_manager,
            snapshot_manager: self.snapshot_manager,
            program_specification: self.program_specification,
            audio_outputs,
            _phantom: PhantomData,
        }
    }
}

pub struct ComponentBuilder<'a, P: Platform, C: Component> {
    machine_builder: &'a mut MachineBuilder<P>,
    component_metadata: &'a mut ComponentMetadata<P>,
    tasks: &'a mut HashMap<TaskId, TaskData>,
    path: &'a ComponentPath,
    _phantom: PhantomData<C>,
}

impl<'a, P: Platform, C: Component> ComponentBuilder<'a, P, C> {
    pub fn path(&self) -> &'a ComponentPath {
        self.path
    }

    pub fn program_manager(&self) -> &Arc<ProgramManager> {
        self.machine_builder.program_manager()
    }

    pub fn memory_access_table(&self) -> Arc<MemoryAccessTable> {
        self.machine_builder.memory_access_table.clone()
    }

    pub fn host_sample_rate(&self) -> Ratio<u32> {
        self.machine_builder.sample_rate
    }

    pub fn registry(&self) -> &ComponentRegistry {
        &self.machine_builder.registry
    }

    pub fn set_lazy_component_initializer(
        self,
        initializer: impl FnOnce(&mut C, &LateInitializedData<P>) + 'static,
    ) -> Self {
        self.component_metadata
            .late_initializer
            .get_or_insert(Box::new(|component, data| {
                let component: &mut C = (component as &mut dyn Any).downcast_mut().unwrap();

                initializer(component, data);
            }));

        self
    }

    pub fn save(&self) -> Option<(Box<dyn Read>, ComponentVersion)> {
        None
    }

    /// Insert a component into the machine
    #[inline]
    pub fn insert_child_component<B: ComponentConfig<P>>(
        self,
        name: &str,
        config: B,
    ) -> (Self, ComponentPath) {
        assert!(
            !name.contains(ComponentPath::SEPARATOR),
            "This function requires a name not a path"
        );

        let mut path = self.path.clone();
        path.push(name).unwrap();

        let mut tasks = HashMap::new();
        let mut component_metadata = ComponentMetadata::default();

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: self.machine_builder,
            component_metadata: &mut component_metadata,
            path: &path,
            tasks: &mut tasks,
            _phantom: PhantomData,
        };

        let component = config
            .build_component(component_builder)
            .expect("Failed to build component");

        self.machine_builder
            .registry
            .insert_component(path.clone(), component, tasks);

        self.machine_builder
            .component_metadata
            .insert(path.clone(), component_metadata);

        (self, path)
    }

    /// Insert a component with a default config
    pub fn insert_default_child_component<B: ComponentConfig<P> + Default>(
        self,
        name: &str,
    ) -> (Self, ComponentPath) {
        let config = B::default();
        self.insert_child_component(name, config)
    }

    pub fn insert_audio_output(self, name: impl Into<Cow<'static, str>>) -> (Self, ResourcePath) {
        let resource_path = ResourcePath {
            component: self.path.clone(),
            name: name.into(),
        };

        self.component_metadata
            .audio_outputs
            .insert(resource_path.clone());

        (self, resource_path)
    }

    pub fn insert_display(self, name: impl Into<Cow<'static, str>>) -> (Self, ResourcePath) {
        let resource_path = ResourcePath {
            component: self.path.clone(),
            name: name.into(),
        };

        self.component_metadata
            .displays
            .insert(resource_path.clone());

        (self, resource_path)
    }

    pub fn insert_gamepad(
        self,
        name: impl Into<Cow<'static, str>>,
        gamepad: Arc<VirtualGamepad>,
    ) -> (Self, ResourcePath) {
        let resource_path = ResourcePath {
            component: self.path.clone(),
            name: name.into(),
        };

        self.component_metadata
            .gamepads
            .insert(resource_path.clone(), gamepad);

        (self, resource_path)
    }

    /// Insert a callback into the memory translation table for reading
    pub fn memory_map_read(
        self,
        range: RangeInclusive<Address>,
        address_space: AddressSpaceId,
    ) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Component {
                range,
                component: self.path.clone(),
                permissions: Permissions {
                    read: true,
                    write: false,
                },
            }],
        );

        self
    }

    pub fn memory_map_write(
        self,
        range: RangeInclusive<Address>,
        address_space: AddressSpaceId,
    ) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Component {
                range,
                component: self.path.clone(),
                permissions: Permissions {
                    read: false,
                    write: true,
                },
            }],
        );

        self
    }

    pub fn memory_map(self, range: RangeInclusive<Address>, address_space: AddressSpaceId) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Component {
                range,
                component: self.path.clone(),
                permissions: Permissions {
                    read: true,
                    write: true,
                },
            }],
        );

        self
    }

    pub fn memory_mirror_map_read(
        self,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
        address_space: AddressSpaceId,
    ) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Mirror {
                source,
                destination,
                permissions: Permissions {
                    read: true,
                    write: false,
                },
            }],
        );

        self
    }

    pub fn memory_mirror_map_write(
        self,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
        address_space: AddressSpaceId,
    ) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Mirror {
                source,
                destination,
                permissions: Permissions {
                    read: false,
                    write: true,
                },
            }],
        );

        self
    }

    pub fn memory_mirror_map(
        self,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
        address_space: AddressSpaceId,
    ) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Mirror {
                source,
                destination,
                permissions: Permissions {
                    read: true,
                    write: true,
                },
            }],
        );

        self
    }

    pub fn insert_task(
        self,
        name: impl Into<Cow<'static, str>>,
        frequency: Ratio<u32>,
        ty: TaskType,
        task: impl Task<C>,
    ) -> (Self, ResourcePath) {
        let resource_path = ResourcePath {
            component: self.path.clone(),
            name: name.into(),
        };

        let (task_id, task_data) = self.machine_builder.scheduler.insert_task(
            resource_path.clone(),
            ty,
            frequency.reduced().recip(),
            task,
        );

        self.tasks.insert(task_id, task_data);

        (self, resource_path)
    }
}
