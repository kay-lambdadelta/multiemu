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
    memory::{AddressSpaceId, MappingPermissions, MemoryAccessTable, MemoryRemappingCommand},
    persistence::{SaveManager, SnapshotManager},
    platform::Platform,
    program::{MachineId, ProgramMetadata, ProgramSpecification},
    scheduler::{ErasedTask, SchedulerState, Task, TaskMut, scheduler_thread},
};
use indexmap::IndexMap;
use num::rational::Ratio;
use rustc_hash::FxBuildHasher;
use std::{
    any::Any,
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Debug,
    io::Read,
    marker::PhantomData,
    ops::{Deref, DerefMut, RangeInclusive},
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

/// Overall data extracted from components needed for machine initialization
pub struct ComponentMetadata<P: Platform> {
    pub tasks: HashMap<ResourcePath, StoredTask>,
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
            tasks: Default::default(),
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
    rom_metadata: Arc<ProgramMetadata>,
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
}

pub struct StoredTask {
    pub period: Ratio<u32>,
    pub task: ErasedTask,
}

impl Debug for StoredTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredTask")
            .field("frequency", &self.period)
            .finish()
    }
}

impl<P: Platform> MachineBuilder<P> {
    pub(crate) fn new(
        program_specification: Option<ProgramSpecification>,
        program_manager: Arc<ProgramMetadata>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
        sample_rate: Ratio<u32>,
    ) -> Self {
        let registry = Arc::new(ComponentRegistry::default());
        let save_manager = SaveManager::new(save_path);
        let snapshot_manager = SnapshotManager::new(snapshot_path);

        MachineBuilder::<P> {
            memory_access_table: Arc::new(MemoryAccessTable::new(registry.clone())),
            save_manager,
            snapshot_manager,
            registry,
            rom_metadata: program_manager,
            sample_rate,
            component_metadata: IndexMap::default(),
            program_specification,
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

    pub fn program_manager(&self) -> &Arc<ProgramMetadata> {
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
        let mut component_metadata = ComponentMetadata::default();

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: self,
            component_metadata: &mut component_metadata,
            path: &path,
            _phantom: PhantomData,
        };

        let component = config
            .build_component(component_builder)
            .expect("Failed to build component");
        self.registry.insert_component(path.clone(), component);

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
            !name.contains(ComponentPath::SEPERATOR),
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
        if width > usize::BITS as u8 {
            panic!(
                "This host machine cannot handle an address space of {} bits",
                width
            );
        }

        let mutable_access_table = Arc::get_mut(&mut self.memory_access_table)
            .expect("Address spaces must be added before memory access table is spread");

        let id = mutable_access_table.insert_address_space(width);

        (self, id)
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
        // NOTE: EVERY test should set this to false
        scheduler_dedicated_thread: bool,
    ) -> Machine<P> {
        let mut tasks = HashMap::default();
        let mut virtual_gamepads = HashMap::default();
        let mut audio_outputs = HashSet::new();
        let mut component_initializers = HashMap::new();
        let mut displays = HashSet::default();

        for (path, component_metadata) in self.component_metadata.drain(..) {
            if let Some(initializer) = component_metadata.late_initializer {
                component_initializers.insert(path, initializer);
            }

            displays.extend(component_metadata.displays);

            // Gather the tasks
            for (resource_path, task) in component_metadata.tasks {
                tasks.insert(resource_path, task);
            }

            virtual_gamepads.extend(component_metadata.gamepads);

            audio_outputs.extend(component_metadata.audio_outputs);
        }

        let late_initialized_data = LateInitializedData::<P> {
            component_graphics_initialization_data,
        };

        for (component_path, initializer) in component_initializers {
            self.registry
                .interact_dyn_mut(&component_path, |component| {
                    initializer(component, &late_initialized_data)
                })
                .unwrap();
        }

        // Create the scheduler
        let scheduler_state = SchedulerState::new(tasks, self.registry.clone());
        let scheduler_handle = scheduler_state.handle();

        let scheduler_state = if scheduler_dedicated_thread {
            std::thread::spawn(|| {
                scheduler_thread(scheduler_state);
            });

            None
        } else {
            Some(scheduler_state)
        };

        Machine {
            scheduler_handle,
            scheduler_state,
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
    path: &'a ComponentPath,
    _phantom: PhantomData<C>,
}

impl<'a, P: Platform, C: Component> ComponentBuilder<'a, P, C> {
    pub fn path(&self) -> &'a ComponentPath {
        self.path
    }

    pub fn program_manager(&self) -> &Arc<ProgramMetadata> {
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

                initializer(component, data)
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
            !name.contains(ComponentPath::SEPERATOR),
            "This function requires a name not a path"
        );

        let mut path = self.path.clone();
        path.push(name).unwrap();
        let mut component_metadata = ComponentMetadata::default();

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: self.machine_builder,
            component_metadata: &mut component_metadata,
            path: &path,
            _phantom: PhantomData,
        };

        let component = config
            .build_component(component_builder)
            .expect("Failed to build component");
        self.machine_builder
            .registry
            .insert_component(path.clone(), component);

        self.machine_builder
            .component_metadata
            .insert(path.clone(), component_metadata);

        (self, path)
    }

    /// Insert a component with a default config
    #[inline]
    pub fn insert_default_child_component<B: ComponentConfig<P> + Default>(
        self,
        name: &str,
    ) -> (Self, ComponentPath) {
        let config = B::default();
        self.insert_child_component(name, config)
    }

    pub fn insert_audio_output(self, name: impl Into<Cow<'static, str>>) -> (Self, ResourcePath) {
        let resource_path = ResourcePath {
            component_path: self.path.clone(),
            name: name.into(),
        };

        self.component_metadata
            .audio_outputs
            .insert(resource_path.clone());

        (self, resource_path)
    }

    pub fn insert_display(self, name: impl Into<Cow<'static, str>>) -> (Self, ResourcePath) {
        let resource_path = ResourcePath {
            component_path: self.path.clone(),
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
            component_path: self.path.clone(),
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
        address_space: AddressSpaceId,
        range: RangeInclusive<usize>,
    ) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Remap {
                range,
                permissions: vec![MappingPermissions::Read],
                component: self.path.clone(),
            }],
        );

        self
    }

    pub fn memory_map_write(
        self,
        address_space: AddressSpaceId,
        range: RangeInclusive<usize>,
    ) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Remap {
                range,
                permissions: vec![MappingPermissions::Write],
                component: self.path.clone(),
            }],
        );

        self
    }

    pub fn memory_map(self, address_space: AddressSpaceId, range: RangeInclusive<usize>) -> Self {
        self.machine_builder.memory_access_table.remap(
            address_space,
            [MemoryRemappingCommand::Remap {
                range,
                permissions: vec![MappingPermissions::Read, MappingPermissions::Write],
                component: self.path.clone(),
            }],
        );

        self
    }

    /// Insert a task to be executed by the scheduler at the given frequency
    pub fn insert_task(
        self,
        name: impl Into<Cow<'static, str>>,
        frequency: Ratio<u32>,
        mut callback: impl Task<C>,
    ) -> Self {
        let resource_path = ResourcePath {
            component_path: self.path.clone(),
            name: name.into(),
        };

        if self.component_metadata.tasks.contains_key(&resource_path) {
            panic!("Task with path {} already exists", resource_path);
        }

        let mut component = None;
        let component_path = self.path.clone();

        self.component_metadata.tasks.insert(
            resource_path,
            StoredTask {
                period: frequency.reduced().recip(),
                task: Box::new(move |component_registry, slice| {
                    let component = component.get_or_insert_with(|| {
                        component_registry.get::<C>(&component_path).unwrap()
                    });

                    let component_guard = component.read();

                    callback.run(component_guard.deref(), slice);
                }),
            },
        );

        self
    }

    pub fn insert_task_mut(
        self,
        name: impl Into<Cow<'static, str>>,
        frequency: Ratio<u32>,
        mut callback: impl TaskMut<C>,
    ) -> Self {
        let resource_path = ResourcePath {
            component_path: self.path.clone(),
            name: name.into(),
        };

        if self.component_metadata.tasks.contains_key(&resource_path) {
            panic!("Task with path {} already exists", resource_path);
        }

        let mut component = None;
        let component_path = self.path.clone();

        self.component_metadata.tasks.insert(
            resource_path,
            StoredTask {
                period: frequency.reduced().recip(),
                task: Box::new(move |component_registry, slice| {
                    let component = component.get_or_insert_with(|| {
                        component_registry.get::<C>(&component_path).unwrap()
                    });

                    let mut component_guard = component.write();

                    callback.run(component_guard.deref_mut(), slice);
                }),
            },
        );

        self
    }
}
