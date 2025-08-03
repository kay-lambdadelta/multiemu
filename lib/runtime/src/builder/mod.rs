use crate::{
    Machine, UserSpecifiedRoms,
    audio::{AudioCallback, AudioOutputId, AudioOutputInfo},
    builder::task::StoredTask,
    component::{
        Component, ComponentConfig, ComponentId, ComponentPath, ComponentRef, ComponentRegistry,
        ComponentVersion, LateInitializedData,
    },
    graphics::{DisplayCallback, DisplayId, DisplayInfo, GraphicsRequirements},
    input::{VirtualGamepad, VirtualGamepadId},
    memory::{Address, AddressSpaceHandle, MemoryAccessTable},
    platform::{Platform, TestPlatform},
    save::{SaveManager, SnapshotManager},
    scheduler::{Scheduler, Task},
    utils::{DirectMainThreadExecutor, MainThreadQueue},
};
use audio::AudioMetadata;
use graphics::GraphicsMetadata;
use indexmap::IndexMap;
use input::InputMetadata;
use multiemu_graphics::GraphicsApi;
use multiemu_rom::{RomManager, System};
use num::rational::Ratio;
use rangemap::RangeInclusiveSet;
use rustc_hash::FxBuildHasher;
use std::{
    collections::HashMap,
    io::Read,
    ops::RangeInclusive,
    str::FromStr,
    sync::{Arc, Mutex},
    vec::Vec,
};
use task::TaskMetadata;

pub mod audio;
pub mod graphics;
pub mod input;
pub mod memory;
pub mod task;

/// Overall data extracted from components needed for machine initialization
pub struct ComponentMetadata<P: Platform> {
    pub task: Option<TaskMetadata>,
    pub graphics: Option<GraphicsMetadata<P::GraphicsApi>>,
    pub input: Option<InputMetadata>,
    pub audio: Option<AudioMetadata<P::SampleFormat>>,
    pub path: ComponentPath,
    pub component_initializer: Option<Box<dyn FnOnce(&ComponentRegistry, &LateInitializedData<P>)>>,
}

/// Builder to produce a machine, definition crates will want to use this
pub struct MachineBuilder<P: Platform> {
    /// Memory translation table
    memory_access_table: Arc<MemoryAccessTable>,
    /// Rom manager
    rom_manager: Arc<RomManager>,
    /// Save manager
    save_manager: Arc<SaveManager>,
    /// Snapshot manager
    snapshot_manager: Arc<SnapshotManager>,
    /// Selected sample rate
    sample_rate: Ratio<u32>,
    /// The store for components
    registry: Arc<ComponentRegistry>,
    /// Component metadata
    component_metadata: IndexMap<ComponentId, ComponentMetadata<P>, FxBuildHasher>,
    /// Roms we were opened with
    user_specified_roms: Option<UserSpecifiedRoms>,
    /// Counter for assigning things
    current_audio_output_id: AudioOutputId,
    current_display_id: DisplayId,
}

impl<P: Platform> MachineBuilder<P> {
    pub fn new(
        user_specified_roms: Option<UserSpecifiedRoms>,
        rom_manager: Arc<RomManager>,
        save_manager: Arc<SaveManager>,
        snapshot_manager: Arc<SnapshotManager>,
        sample_rate: Ratio<u32>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> Self {
        let main_thread_queue = MainThreadQueue::new(main_thread_executor);
        let component_store = ComponentRegistry::new(main_thread_queue.clone());

        MachineBuilder::<P> {
            current_audio_output_id: AudioOutputId(0),
            current_display_id: DisplayId(0),
            memory_access_table: Arc::new(MemoryAccessTable::new(component_store.clone())),
            save_manager,
            snapshot_manager,
            registry: component_store,
            rom_manager,
            sample_rate,
            component_metadata: IndexMap::default(),
            user_specified_roms,
        }
    }

    pub fn system(&self) -> Option<System> {
        self.user_specified_roms
            .as_ref()
            .map(|roms| roms.main.identity.system())
    }

    pub fn user_specified_roms(&self) -> Option<&UserSpecifiedRoms> {
        self.user_specified_roms.as_ref()
    }

    pub fn rom_manager(&self) -> &Arc<RomManager> {
        &self.rom_manager
    }

    pub fn save_manager(&self) -> &Arc<SaveManager> {
        &self.save_manager
    }

    pub fn snapshot_manager(&self) -> &Arc<SnapshotManager> {
        &self.snapshot_manager
    }

    #[inline]
    fn insert_component_with_path<B: ComponentConfig<P>>(
        mut self,
        path: ComponentPath,
        config: B,
    ) -> (Self, ComponentRef<B::Component>) {
        let component_id = self.registry.generate_id();
        let component_ref = ComponentRef::new(self.registry.clone(), component_id);

        self.component_metadata.insert(
            component_id,
            ComponentMetadata {
                task: Default::default(),
                graphics: Default::default(),
                input: Default::default(),
                audio: Default::default(),
                path,
                component_initializer: None,
            },
        );

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: &mut self,
            component_ref: component_ref.clone(),
        };

        config
            .build_component(component_builder)
            .expect("Failed to build component");

        (self, component_ref)
    }

    /// Insert a component into the machine
    #[inline]
    pub fn insert_component<B: ComponentConfig<P>>(
        self,
        name: &str,
        config: B,
    ) -> (Self, ComponentRef<B::Component>) {
        assert!(
            !name.contains(ComponentPath::SEPERATOR),
            "This function requires a name not a path"
        );

        let path = ComponentPath::from_str(name).unwrap();

        self.insert_component_with_path(path, config)
    }

    /// Insert a component with a default config
    #[inline]
    pub fn insert_default_component<B: ComponentConfig<P> + Default>(
        self,
        name: &str,
    ) -> (Self, ComponentRef<B::Component>) {
        let config = B::default();
        self.insert_component(name, config)
    }

    /// Insert the required information to construct a address space
    pub fn insert_address_space(self, width: u8) -> (Self, AddressSpaceHandle) {
        let id = self.memory_access_table.insert_address_space(width);

        (self, id)
    }

    pub fn graphics_requirements(&self) -> GraphicsRequirements<P::GraphicsApi> {
        self.component_metadata
            .values()
            .filter_map(|metadata| {
                metadata
                    .graphics
                    .as_ref()
                    .map(|gm| &gm.graphics_requirements)
            })
            .fold(GraphicsRequirements::default(), |acc, value| {
                acc | value.clone()
            })
    }

    /// Build the machine
    pub fn build(
        mut self,
        component_graphics_initialization_data: <P::GraphicsApi as GraphicsApi>::InitializationData,
    ) -> Machine<P> {
        let runtime_essentials = LateInitializedData::<P> {
            component_graphics_initialization_data,
        };

        let mut tasks: HashMap<_, HashMap<_, _>> = HashMap::default();
        let mut virtual_gamepads = Vec::default();
        let mut audio_outputs = HashMap::default();
        let mut displays = HashMap::default();

        for (component_id, mut component_metadata) in self.component_metadata.drain(..) {
            if let Some(component_initializer) = component_metadata.component_initializer.take() {
                component_initializer(&self.registry, &runtime_essentials);
            } else {
                assert!(
                    self.registry.contains(component_id),
                    "Component did not insert or lazily insert itself",
                );
            }

            // Gather the framebuffers
            if let Some(graphics_metadata) = component_metadata.graphics {
                displays.extend(graphics_metadata.displays);
            }

            // Gather the tasks
            if let Some(task_metadata) = component_metadata.task {
                for (task_name, task) in task_metadata.tasks {
                    tasks
                        .entry(component_id)
                        .or_default()
                        .insert(task_name, task);
                }
            }

            if let Some(input_metadata) = component_metadata.input {
                virtual_gamepads.extend(input_metadata.gamepads);
            }

            if let Some(audio_metadata) = component_metadata.audio {
                audio_outputs.extend(audio_metadata.audio_outputs);
            }
        }

        // Create the scheduler
        let scheduler = Scheduler::new(tasks);
        let debt_clearer = scheduler.get_debt_clearer();

        // Give the component store a handle to the scheduler
        self.registry.set_debt_clearer(debt_clearer);

        Machine {
            scheduler: Mutex::new(scheduler),
            memory_access_table: self.memory_access_table.clone(),
            virtual_gamepads: virtual_gamepads
                .into_iter()
                .enumerate()
                .map(|(index, gamepad)| {
                    (
                        VirtualGamepadId(
                            index
                                .try_into()
                                .expect("How do you have this many gamepads"),
                        ),
                        gamepad,
                    )
                })
                .collect(),
            component_registry: self.registry,
            displays,
            audio_outputs,
            save_manager: self.save_manager,
            snapshot_manager: self.snapshot_manager,
            user_specified_roms: self.user_specified_roms,
        }
    }
}

impl MachineBuilder<TestPlatform> {
    pub fn new_test(
        user_specified_roms: Option<UserSpecifiedRoms>,
        rom_manager: Arc<RomManager>,
        save_manager: Arc<SaveManager>,
        snapshot_manager: Arc<SnapshotManager>,
    ) -> Self {
        Self::new(
            user_specified_roms,
            rom_manager,
            save_manager,
            snapshot_manager,
            Ratio::from_integer(44100),
            Arc::new(DirectMainThreadExecutor),
        )
    }

    pub fn new_test_minimal() -> Self {
        Self::new(
            None,
            Arc::new(RomManager::new(None, None).unwrap()),
            Arc::new(SaveManager::new(None)),
            Arc::new(SnapshotManager::new(None)),
            Ratio::from_integer(44100),
            Arc::new(DirectMainThreadExecutor),
        )
    }
}

pub struct ComponentBuilder<'a, P: Platform, C: Component> {
    machine_builder: &'a mut MachineBuilder<P>,
    component_ref: ComponentRef<C>,
}

impl<'a, P: Platform, C: Component> ComponentBuilder<'a, P, C> {
    fn metadata(&self) -> &ComponentMetadata<P> {
        self.machine_builder
            .component_metadata
            .get(&self.component_ref.id())
            .unwrap()
    }

    fn metadata_mut(&mut self) -> &mut ComponentMetadata<P> {
        self.machine_builder
            .component_metadata
            .get_mut(&self.component_ref.id())
            .unwrap()
    }

    pub fn path(&self) -> &ComponentPath {
        &self.metadata().path
    }

    pub fn rom_manager(&self) -> &Arc<RomManager> {
        self.machine_builder.rom_manager()
    }

    pub fn memory_access_table(&self) -> Arc<MemoryAccessTable> {
        self.machine_builder.memory_access_table.clone()
    }

    pub fn sample_rate(&self) -> Ratio<u32> {
        self.machine_builder.sample_rate
    }

    /// Accessing this ref the function gives out will panic if the machine isn't complete
    pub fn component_ref(&self) -> ComponentRef<C> {
        self.component_ref.clone()
    }

    pub fn save(&self) -> Option<(Box<dyn Read>, ComponentVersion)> {
        if let Some(main) = self
            .machine_builder
            .user_specified_roms
            .as_ref()
            .map(|roms| &roms.main)
        {
            let metadata = self.metadata();

            self.machine_builder
                .save_manager
                .get(main.id, main.identity.name(), metadata.path.clone())
                .unwrap()
        } else {
            None
        }
    }

    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    pub fn build_local(self, component: C) {
        let path = self.metadata().path.clone();
        let component_id = self.component_ref.id();

        self.machine_builder
            .registry
            .insert_component_local(path, component_id, component);
    }

    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    pub fn build_local_lazy(
        mut self,
        callback: impl FnOnce(&LateInitializedData<P>) -> C + 'static,
    ) {
        let path = self.metadata().path.clone();
        let component_id = self.component_ref.id();

        self.metadata_mut().component_initializer =
            Some(Box::new(move |registry, runtime_essentials| {
                registry.insert_component_local(path, component_id, callback(runtime_essentials));
            }));
    }

    /// Insert this component in the global store, ensuring quick access for all other components
    ///
    /// Use this if unsure
    pub fn build(self, component: C)
    where
        C: Send + Sync,
    {
        let path = self.metadata().path.clone();
        let component_id = self.component_ref.id();

        self.machine_builder
            .registry
            .insert_component(path, component_id, component);
    }

    pub fn build_lazy(mut self, callback: impl FnOnce(&LateInitializedData<P>) -> C + 'static)
    where
        C: Send + Sync,
    {
        let path = self.metadata().path.clone();
        let component_id = self.component_ref.id();

        self.metadata_mut().component_initializer =
            Some(Box::new(move |registry, runtime_essentials| {
                registry.insert_component(path, component_id, callback(runtime_essentials));
            }));
    }

    /// Insert a component into the machine
    #[inline]
    pub fn insert_child_component<B: ComponentConfig<P>>(
        mut self,
        name: &str,
        config: B,
    ) -> (Self, ComponentRef<B::Component>) {
        assert!(
            !name.contains(ComponentPath::SEPERATOR),
            "This function requires a name not a path"
        );

        let mut path = self.metadata().path.clone();
        path.push(name).unwrap();

        let component_id = self.machine_builder.registry.generate_id();
        let component_ref = ComponentRef::new(self.machine_builder.registry.clone(), component_id);

        self.machine_builder.component_metadata.insert(
            component_id,
            ComponentMetadata {
                task: Default::default(),
                graphics: Default::default(),
                input: Default::default(),
                audio: Default::default(),
                path,
                component_initializer: None,
            },
        );

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: &mut self.machine_builder,
            component_ref: component_ref.clone(),
        };

        config
            .build_component(component_builder)
            .expect("Failed to build component");

        (self, component_ref)
    }

    /// Insert a component with a default config
    #[inline]
    pub fn insert_default_child_component<B: ComponentConfig<P> + Default>(
        self,
        name: &str,
    ) -> (Self, ComponentRef<B::Component>) {
        let config = B::default();
        self.insert_child_component(name, config)
    }

    pub fn insert_audio_output(
        mut self,
        callback: impl AudioCallback<P::SampleFormat>,
    ) -> (Self, AudioOutputId) {
        let audio_output_id = AudioOutputId(self.machine_builder.current_audio_output_id.0);
        self.machine_builder.current_audio_output_id.0 = self
            .machine_builder
            .current_audio_output_id
            .0
            .checked_add(1)
            .expect("Too many audio outputs");

        self.metadata_mut()
            .audio
            .get_or_insert_default()
            .audio_outputs
            .insert(
                audio_output_id,
                AudioOutputInfo {
                    callback: Box::new(callback),
                },
            );

        (self, audio_output_id)
    }

    pub fn insert_display(
        mut self,
        callback: impl DisplayCallback<P::GraphicsApi>,
    ) -> (Self, DisplayId) {
        let display_id = DisplayId(self.machine_builder.current_display_id.0);
        self.machine_builder.current_display_id.0 = self
            .machine_builder
            .current_display_id
            .0
            .checked_add(1)
            .expect("Too many displays");

        let metadata = self.metadata_mut().graphics.get_or_insert_default();

        metadata.displays.insert(
            display_id,
            DisplayInfo {
                callback: Box::new(callback),
            },
        );

        (self, display_id)
    }

    pub fn insert_gamepad(self, gamepad: Arc<VirtualGamepad>) -> Self {
        self.machine_builder
            .component_metadata
            .get_mut(&self.component_ref.id())
            .unwrap()
            .input
            .get_or_insert_default()
            .gamepads
            .push(gamepad);

        self
    }

    /// Insert a callback into the memory translation table for reading
    pub fn map_memory_read(
        self,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> Self {
        // Merge all the addresses together so we can remap them without erasing previous ones
        // TODO: Explore remapping without erasing old entires? Hard?
        let mut merged_addresses: HashMap<_, RangeInclusiveSet<_>> = HashMap::new();

        for (address_space, assigned_addresses) in assigned_addresses {
            merged_addresses
                .entry(address_space)
                .or_default()
                .insert(assigned_addresses);
        }

        for (address_space, address_range) in merged_addresses {
            self.machine_builder.memory_access_table.remap_read_memory(
                self.component_ref.id(),
                address_space,
                address_range,
            );
        }

        self
    }

    pub fn map_memory_write(
        self,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> Self {
        let mut merged_addresses: HashMap<_, RangeInclusiveSet<_>> = HashMap::new();

        for (address_space, assigned_addresses) in assigned_addresses {
            merged_addresses
                .entry(address_space)
                .or_default()
                .insert(assigned_addresses);
        }

        for (address_space, address_range) in merged_addresses {
            self.machine_builder.memory_access_table.remap_write_memory(
                self.component_ref.id(),
                address_space,
                address_range,
            );
        }

        self
    }

    pub fn map_memory(
        self,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> Self {
        let mut merged_addresses: HashMap<_, RangeInclusiveSet<_>> = HashMap::new();

        for (address_space, assigned_addresses) in assigned_addresses {
            merged_addresses
                .entry(address_space)
                .or_default()
                .insert(assigned_addresses);
        }

        for (address_space, address_range) in merged_addresses {
            self.machine_builder.memory_access_table.remap_memory(
                self.component_ref.id(),
                address_space,
                address_range,
            );
        }

        self
    }

    /// Insert a task to be executed by the scheduler at the given frequency
    pub fn insert_task(mut self, frequency: Ratio<u32>, name: &str, callback: impl Task) -> Self {
        let task_metatada = self.metadata_mut().task.get_or_insert_default();

        if task_metatada.tasks.contains_key(name) {
            panic!("Task with name {} already exists", name);
        }

        task_metatada.tasks.insert(
            name.to_string(),
            StoredTask {
                period: frequency.reduced().recip(),
                lazy: false,
                task: Box::new(callback),
            },
        );

        self
    }

    /// Insert a task to be executed by the scheduler at the given frequency
    ///
    /// This task will be lazily executed, i.e. only when the component that inserted it actually interacts with it
    ///
    /// As such, any side effects of the task may be out of order (apart from interactions with the component that inserted it), so use it for tasks that have no side effects
    pub fn insert_lazy_task(
        mut self,
        frequency: Ratio<u32>,
        name: &str,
        callback: impl Task,
    ) -> Self {
        let task_metatada = self.metadata_mut().task.get_or_insert_default();

        if task_metatada.tasks.contains_key(name) {
            panic!("Task with name {} already exists", name);
        }

        task_metatada.tasks.insert(
            name.to_string(),
            StoredTask {
                period: frequency.reduced().recip(),
                lazy: true,
                task: Box::new(callback),
            },
        );

        self
    }
}
