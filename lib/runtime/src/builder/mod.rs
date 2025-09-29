use crate::{
    Machine, UserSpecifiedRoms,
    audio::AudioOutputId,
    builder::{memory::MemoryMetadata, task::StoredTask},
    component::{
        Component, ComponentConfig, ComponentId, ComponentPath, ComponentRef, ComponentRegistry,
        ComponentVersion, LateInitializedData,
    },
    graphics::{DisplayId, FramebufferStorage, GraphicsRequirements},
    input::{VirtualGamepad, VirtualGamepadId},
    memory::{AddressSpaceId, MemoryAccessTable, MemoryRemappingCommands, MemoryType},
    platform::Platform,
    save::{SaveManager, SnapshotManager},
    scheduler::{Scheduler, Task},
    utils::MainThreadQueue,
};
use arc_swap::ArcSwapOption;
use audio::AudioMetadata;
use graphics::GraphicsMetadata;
use indexmap::IndexMap;
use input::InputMetadata;
use multiemu_graphics::GraphicsApi;
use multiemu_rom::{RomMetadata, System};
use num::rational::Ratio;
use rustc_hash::FxBuildHasher;
use std::{
    borrow::Cow,
    collections::HashMap,
    io::Read,
    ops::RangeInclusive,
    path::PathBuf,
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
    pub audio: Option<AudioMetadata>,
    pub memory: Option<MemoryMetadata>,
    pub path: ComponentPath,
    pub component_initializer: Option<Box<dyn FnOnce(&LateInitializedData<P>)>>,
}

/// Builder to produce a machine, definition crates will want to use this
pub struct MachineBuilder<P: Platform> {
    /// Memory translation table
    memory_access_table: Arc<MemoryAccessTable>,
    /// Rom manager
    rom_metadata: Arc<RomMetadata>,
    /// Save manager
    save_manager: SaveManager,
    /// Snapshot manager
    snapshot_manager: SnapshotManager,
    /// Graphics manager
    graphics_manager: FramebufferStorage<P::GraphicsApi>,
    /// Selected sample rate
    sample_rate: Ratio<u32>,
    /// The store for components
    registry: Arc<ComponentRegistry>,
    /// Component metadata
    component_metadata: IndexMap<ComponentId, ComponentMetadata<P>, FxBuildHasher>,
    /// Roms we were opened with
    user_specified_roms: Option<UserSpecifiedRoms>,
}

impl<P: Platform> MachineBuilder<P> {
    pub(crate) fn new(
        user_specified_roms: Option<UserSpecifiedRoms>,
        rom_manager: Arc<RomMetadata>,
        save_path: Option<PathBuf>,
        snapshot_path: Option<PathBuf>,
        sample_rate: Ratio<u32>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> Self {
        let main_thread_queue = MainThreadQueue::new(main_thread_executor);
        let component_store = ComponentRegistry::new(main_thread_queue.clone());
        let save_manager = SaveManager::new(save_path);
        let snapshot_manager = SnapshotManager::new(snapshot_path);

        MachineBuilder::<P> {
            memory_access_table: Arc::new(MemoryAccessTable::new(component_store.clone())),
            save_manager,
            snapshot_manager,
            registry: component_store,
            rom_metadata: rom_manager,
            sample_rate,
            component_metadata: IndexMap::default(),
            user_specified_roms,
            graphics_manager: FramebufferStorage::default(),
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

    pub fn rom_manager(&self) -> &Arc<RomMetadata> {
        &self.rom_metadata
    }

    #[inline]
    fn insert_component_with_path<B: ComponentConfig<P>>(
        mut self,
        path: ComponentPath,
        config: B,
    ) -> (Self, ComponentRef<B::Component>) {
        let component_ref = ComponentRef::new(self.registry.clone());
        let mut component_metadata = ComponentMetadata {
            task: Default::default(),
            graphics: Default::default(),
            input: Default::default(),
            audio: Default::default(),
            memory: Default::default(),
            path: path.clone(),
            component_initializer: None,
        };

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: &mut self,
            component_ref: component_ref.clone(),
            component_metadata: &mut component_metadata,
        };

        config
            .build_component(component_builder)
            .expect("Failed to build component");

        self.component_metadata
            .insert(component_ref.id(), component_metadata);

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
        let graphics_manager = Arc::new(self.graphics_manager);

        let runtime_essentials = LateInitializedData::<P> {
            graphics_manager: graphics_manager.clone(),
        };

        let mut tasks: HashMap<_, HashMap<_, _>> = HashMap::default();
        let mut virtual_gamepads = Vec::default();
        let mut audio_outputs = HashMap::default();
        let mut displays = HashMap::default();

        for (component_id, mut component_metadata) in self.component_metadata.drain(..) {
            if let Some(component_initializer) = component_metadata.component_initializer.take() {
                component_initializer(&runtime_essentials);
            }

            if let Some(memory_metadata) = component_metadata.memory {
                let mut collected_remappings_commands: HashMap<_, Vec<_>> = HashMap::default();

                for (address_space, commands) in memory_metadata
                    .read
                    .into_iter()
                    .flat_map(|(address_space, addresses)| {
                        addresses.into_iter().map(move |addresses| {
                            (
                                address_space,
                                MemoryRemappingCommands::AddComponent {
                                    range: addresses,
                                    component_id,
                                    types: vec![MemoryType::Read],
                                },
                            )
                        })
                    })
                    .chain(memory_metadata.write.into_iter().flat_map(
                        |(address_space, addresses)| {
                            addresses.into_iter().map(move |addresses| {
                                (
                                    address_space,
                                    MemoryRemappingCommands::AddComponent {
                                        range: addresses,
                                        component_id,
                                        types: vec![MemoryType::Write],
                                    },
                                )
                            })
                        },
                    ))
                {
                    collected_remappings_commands
                        .entry(address_space)
                        .or_default()
                        .push(commands);
                }

                for (address_space, commands) in collected_remappings_commands {
                    self.memory_access_table.remap(address_space, commands);
                }
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
            graphics_manager,
            save_manager: self.save_manager,
            snapshot_manager: self.snapshot_manager,
            user_specified_roms: self.user_specified_roms,
        }
    }
}

pub struct ComponentBuilder<'a, P: Platform, C: Component> {
    machine_builder: &'a mut MachineBuilder<P>,
    component_ref: ComponentRef<C>,
    component_metadata: &'a mut ComponentMetadata<P>,
}

impl<'a, P: Platform, C: Component> ComponentBuilder<'a, P, C> {
    pub fn path(&self) -> &ComponentPath {
        &self.component_metadata.path
    }

    pub fn rom_manager(&self) -> &Arc<RomMetadata> {
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

    pub fn set_lazy_component_initializer(
        self,
        initializer: impl FnOnce(&LateInitializedData<P>) + 'static,
    ) -> Self {
        self.component_metadata
            .component_initializer
            .get_or_insert(Box::new(initializer));

        self
    }

    pub fn save(&self) -> Option<(Box<dyn Read>, ComponentVersion)> {
        if let Some(main) = self
            .machine_builder
            .user_specified_roms
            .as_ref()
            .map(|roms| &roms.main)
        {
            let metadata = &*self.component_metadata;

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
        let path = self.component_metadata.path.clone();

        let id = self
            .machine_builder
            .registry
            .insert_component_local(path, component);

        self.component_ref.set_id(id);
    }

    /// Insert this component in the global store, ensuring quick access for all other components
    ///
    /// Use this if unsure
    pub fn build(self, component: C)
    where
        C: Send + Sync,
    {
        let path = self.component_metadata.path.clone();

        let id = self
            .machine_builder
            .registry
            .insert_component(path, component);

        self.component_ref.set_id(id);
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

        let mut path = self.component_metadata.path.clone();
        path.push(name).unwrap();

        let mut component_metadata = ComponentMetadata {
            task: Default::default(),
            graphics: Default::default(),
            input: Default::default(),
            audio: Default::default(),
            memory: Default::default(),
            path: path.clone(),
            component_initializer: None,
        };

        let component_ref = ComponentRef::new(self.machine_builder.registry.clone());

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: &mut self.machine_builder,
            component_ref: component_ref.clone(),
            component_metadata: &mut component_metadata,
        };

        config
            .build_component(component_builder)
            .expect("Failed to build component");

        self.machine_builder
            .component_metadata
            .insert(component_ref.id(), component_metadata);

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
        self,
        name: impl Into<Cow<'static, str>>,
        callback: impl AudioCallback<P::SampleFormat>,
    ) -> (Self, AudioOutputId) {
        let name = name.into();

        let audio_output_id = AudioOutputId(name);

        self.component_metadata
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
        self,
    ) -> (
        Self,
        ArcSwapOption<<P::GraphicsApi as GraphicsApi>::FramebufferTexture>,
    ) {
        let display_id = DisplayId(self.machine_builder.current_display_id.0);
        self.machine_builder.current_display_id.0 = self
            .machine_builder
            .current_display_id
            .0
            .checked_add(1)
            .expect("Too many displays");

        let metadata = self.component_metadata.graphics.get_or_insert_default();

        metadata.displays.insert(
            display_id,
            DisplayInfo {
                display: Box::new(callback),
            },
        );

        (self, display_id)
    }

    pub fn insert_gamepad(self, gamepad: Arc<VirtualGamepad>) -> Self {
        self.component_metadata
            .input
            .get_or_insert_default()
            .gamepads
            .push(gamepad);

        self
    }

    /// Insert a callback into the memory translation table for reading
    pub fn memory_map_read(
        self,
        address_space: AddressSpaceId,
        addresses: RangeInclusive<usize>,
    ) -> Self {
        self.component_metadata
            .memory
            .get_or_insert_default()
            .read
            .entry(address_space)
            .or_default()
            .insert(addresses);

        self
    }

    pub fn memory_map_write(
        self,
        address_space: AddressSpaceId,
        addresses: RangeInclusive<usize>,
    ) -> Self {
        self.component_metadata
            .memory
            .get_or_insert_default()
            .write
            .entry(address_space)
            .or_default()
            .insert(addresses);

        self
    }

    pub fn memory_map(
        self,
        address_space: AddressSpaceId,
        addresses: RangeInclusive<usize>,
    ) -> Self {
        let component_metadata = self.component_metadata.memory.get_or_insert_default();

        component_metadata
            .read
            .entry(address_space)
            .or_default()
            .insert(addresses.clone());

        component_metadata
            .write
            .entry(address_space)
            .or_default()
            .insert(addresses);

        self
    }

    /// Insert a task to be executed by the scheduler at the given frequency
    pub fn insert_task(self, frequency: Ratio<u32>, name: &str, callback: impl Task) -> Self {
        let task_metatada = self.component_metadata.task.get_or_insert_default();

        if task_metatada.tasks.contains_key(name) {
            panic!("Task with name {} already exists", name);
        }

        task_metatada.tasks.insert(
            name.to_string(),
            StoredTask {
                period: frequency.reduced().recip(),
                task: Box::new(callback),
            },
        );

        self
    }
}
