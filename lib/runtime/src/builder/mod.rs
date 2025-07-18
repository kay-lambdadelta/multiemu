use crate::{
    Machine, UserSpecifiedRoms,
    audio::{AudioCallback, AudioOutputId, AudioOutputInfo},
    builder::task::StoredTask,
    component::{
        Component, ComponentConfig, ComponentId, ComponentRef, ComponentRegistry, RuntimeEssentials,
    },
    graphics::{DisplayCallback, DisplayId, DisplayInfo, GraphicsRequirements},
    input::{VirtualGamepad, VirtualGamepadId},
    memory::{Address, AddressSpaceHandle, MemoryAccessTable},
    platform::{Platform, TestPlatform},
    scheduler::{Scheduler, Task},
    utils::{DirectMainThreadExecutor, MainThreadQueue},
};
use audio::AudioMetadata;
use graphics::GraphicsMetadata;
use input::InputMetadata;
use multiemu_graphics::GraphicsApi;
use multiemu_rom::{RomManager, System};
use multiemu_save::{ComponentName, SaveManager, SnapshotManager};
use num::rational::Ratio;
use pathfinding::prelude::topological_sort;
use rangemap::RangeInclusiveSet;
use rustc_hash::FxBuildHasher;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    marker::PhantomData,
    ops::RangeInclusive,
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
    pub dependencies: HashSet<ComponentId>,
}

impl<P: Platform> Default for ComponentMetadata<P> {
    fn default() -> Self {
        Self {
            task: None,
            graphics: None,
            input: None,
            audio: None,
            dependencies: HashSet::new(),
        }
    }
}

pub type ComponentBuilderCallback<P> =
    Box<dyn FnOnce(&mut MachineBuilder<P>, &RuntimeEssentials<P>)>;

/// Builder to produce a machine, definition crates will want to use this
pub struct MachineBuilder<P: Platform> {
    /// Memory translation table
    memory_translation_table: Arc<MemoryAccessTable>,
    /// Rom manager
    rom_manager: Arc<RomManager>,
    /// Save manager
    save_manager: Arc<SaveManager>,
    /// Snapshot manager
    snapshot_manager: Arc<SnapshotManager>,
    /// Selected sample rate
    sample_rate: Ratio<u32>,
    /// The store for components
    component_store: Arc<ComponentRegistry>,
    /// Stored component builder callbacks for late initialization
    component_builders: BTreeMap<ComponentId, ComponentBuilderCallback<P>>,
    /// Graphics requirements
    graphics_requirements: GraphicsRequirements<P::GraphicsApi>,
    /// Component metadata
    component_metadata: HashMap<ComponentId, ComponentMetadata<P>, FxBuildHasher>,
    /// Roms we were opened with
    user_specified_roms: Option<UserSpecifiedRoms>,
    /// System this is
    system: System,
    /// Counter for assigning things
    current_audio_output_id: AudioOutputId,
    current_display_id: DisplayId,
}

impl<P: Platform> MachineBuilder<P> {
    pub fn new(
        user_specified_roms: Option<UserSpecifiedRoms>,
        system: System,
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
            component_builders: BTreeMap::new(),
            memory_translation_table: Arc::new(MemoryAccessTable::new(component_store.clone())),
            save_manager,
            snapshot_manager,
            component_store,
            rom_manager,
            sample_rate,
            graphics_requirements: GraphicsRequirements::default(),
            component_metadata: HashMap::default(),
            user_specified_roms,
            system,
        }
    }

    pub fn system(&self) -> System {
        self.system
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

    /// Insert a component into the machine
    #[inline]
    pub fn insert_component<B: ComponentConfig<P>>(
        self,
        name: &str,
        config: B,
    ) -> (Self, ComponentRef<B::Component>) {
        self.insert_component_with_dependencies(name, config, [])
    }

    pub fn insert_component_with_dependencies<B: ComponentConfig<P>>(
        mut self,
        name: &str,
        config: B,
        dependencies: impl IntoIterator<Item = ComponentId>,
    ) -> (Self, ComponentRef<B::Component>) {
        let name: ComponentName = name.parse().unwrap();

        let component_id = self.component_store.generate_id();
        let component_ref = ComponentRef::new(self.component_store.clone(), component_id);

        self.component_metadata.insert(
            component_id,
            ComponentMetadata {
                dependencies: config
                    .build_dependencies()
                    .into_iter()
                    .chain(dependencies)
                    .collect(),
                ..Default::default()
            },
        );

        self.component_builders.insert(component_id, {
            let component_ref = component_ref.clone();

            Box::new(move |machine_builder, runtime_essentials| {
                let save = if let Some(main) = machine_builder
                    .user_specified_roms
                    .as_ref()
                    .map(|roms| roms.main)
                {
                    machine_builder.save_manager.get(main).unwrap()
                } else {
                    None
                };
                let component_save = save.as_ref().and_then(|save| save.components.get(&name));

                let component_builder = ComponentBuilder::<P, B::Component> {
                    runtime_essentials,
                    machine_builder,
                    component_id,
                    name: name.clone(),
                    _phantom: PhantomData,
                };

                config
                    .build_component(component_ref.clone(), component_builder, component_save)
                    .expect("Failed to build component");
            })
        });

        (self, component_ref)
    }

    /// Insert a component with a default config
    pub fn insert_default_component<B: ComponentConfig<P> + Default>(
        self,
        name: &str,
    ) -> (Self, ComponentRef<B::Component>) {
        let config = B::default();
        self.insert_component(name, config)
    }

    /// Insert the required information to construct a address space
    pub fn insert_address_space(self, width: u8) -> (Self, AddressSpaceHandle) {
        let id = self.memory_translation_table.insert_address_space(width);

        (self, id)
    }

    pub fn graphics_requirements(&self) -> GraphicsRequirements<P::GraphicsApi> {
        self.graphics_requirements.clone()
    }

    /// Build the machine
    pub fn build(
        mut self,
        component_graphics_initialization_data: <P::GraphicsApi as GraphicsApi>::InitializationData,
    ) -> Machine<P> {
        let component_ids: Vec<_> = self.component_builders.keys().copied().collect();

        let initialization_order = topological_sort(&component_ids, |component_id| {
            self.component_metadata
                .get(component_id)
                .map(|metadata| &metadata.dependencies)
                .unwrap()
                .iter()
                .copied()
        })
        .expect("Cyclic dependency detected");

        let runtime_essentials = RuntimeEssentials {
            memory_access_table: self.memory_translation_table.clone(),
            rom_manager: self.rom_manager.clone(),
            component_graphics_initialization_data,
            sample_rate: self.sample_rate,
        };

        for component_id in initialization_order.into_iter().rev() {
            let component_builder = self.component_builders.remove(&component_id).unwrap();
            component_builder(&mut self, &runtime_essentials);

            let component_name = self
                .component_store
                .get_name(component_id)
                .expect("Component did not insert itself");

            tracing::debug!("Set up component: {:?}", component_name);
        }

        let mut tasks: HashMap<_, Vec<_>> = HashMap::new();
        let mut virtual_gamepads = Vec::default();
        let mut audio_outputs = HashMap::default();
        let mut displays = HashMap::default();

        for (component_id, component_metadata) in self.component_metadata.drain() {
            // Gather the framebuffers
            if let Some(graphics_metadata) = component_metadata.graphics {
                displays.extend(graphics_metadata.displays);
            }

            // Gather the tasks
            if let Some(task_metadata) = component_metadata.task {
                for task in task_metadata.tasks {
                    tasks.entry(component_id).or_default().push(task);
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
        self.component_store.set_debt_clearer(debt_clearer);

        // Make sure all the components do their proper post initialization
        self.component_store.interact_all(|compoenent| {
            compoenent.runtime_ready();
        });

        Machine {
            scheduler: Mutex::new(scheduler),
            memory_access_table: self.memory_translation_table.clone(),
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
            component_registry: self.component_store,
            displays,
            audio_outputs,
            rom_manager: self.rom_manager,
            save_manager: self.save_manager,
            snapshot_manager: self.snapshot_manager,
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
            System::Unknown,
            rom_manager,
            save_manager,
            snapshot_manager,
            Ratio::from_integer(441000),
            Arc::new(DirectMainThreadExecutor),
        )
    }

    pub fn new_test_minimal() -> Self {
        Self::new(
            None,
            System::Unknown,
            Arc::new(RomManager::new(None, None).unwrap()),
            Arc::new(SaveManager::new(None)),
            Arc::new(SnapshotManager::new(None)),
            Ratio::from_integer(441000),
            Arc::new(DirectMainThreadExecutor),
        )
    }
}

/// Struct passed into components for their initialization purposes. Do not refer to this directly.
pub struct ComponentBuilder<'a, P: Platform, C: Component> {
    runtime_essentials: &'a RuntimeEssentials<P>,
    machine_builder: &'a mut MachineBuilder<P>,
    component_id: ComponentId,
    name: ComponentName,
    _phantom: PhantomData<C>,
}

impl<'a, P: Platform, C: Component> ComponentBuilder<'a, P, C> {
    fn metadata(&mut self) -> &mut ComponentMetadata<P> {
        self.machine_builder
            .component_metadata
            .get_mut(&self.component_id)
            .unwrap()
    }

    pub fn essentials(&self) -> &RuntimeEssentials<P> {
        &self.runtime_essentials
    }

    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    pub fn build(self, component: C) {
        self.machine_builder.component_store.insert_component(
            self.name,
            self.component_id,
            component,
        );
    }

    /// Insert this component in the global store, ensuring quick access for all other components
    ///
    /// Use this if unsure
    pub fn build_global(self, component: C)
    where
        C: Send + Sync,
    {
        self.machine_builder
            .component_store
            .insert_component_global(self.name, self.component_id, component);
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

        self.metadata()
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

        let metadata = self.metadata().graphics.get_or_insert_default();

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
            .get_mut(&self.component_id)
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
            self.machine_builder
                .memory_translation_table
                .remap_read_memory(self.component_id, address_space, address_range);
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
            self.machine_builder
                .memory_translation_table
                .remap_write_memory(self.component_id, address_space, address_range);
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
            self.machine_builder.memory_translation_table.remap_memory(
                self.component_id,
                address_space,
                address_range,
            );
        }

        self
    }

    /// Insert a task to be executed by the scheduler at the given frequency
    pub fn insert_task(mut self, frequency: Ratio<u32>, callback: impl Task) -> Self {
        let task_metatada = self.metadata().task.get_or_insert_default();

        task_metatada.tasks.push(StoredTask {
            period: frequency.reduced().recip(),
            lazy: false,
            task: Box::new(callback),
        });

        self
    }

    /// Insert a task to be executed by the scheduler at the given frequency
    ///
    /// This task will be lazily executed, i.e. only when the component that inserted it actually interacts with it
    ///
    /// As such, any side effects of the task may be out of order (apart from interactions with the component that inserted it), so use it for tasks that have no side effects
    pub fn insert_lazy_task(mut self, frequency: Ratio<u32>, callback: impl Task) -> Self {
        let task_metatada = self.metadata().task.get_or_insert_default();

        task_metatada.tasks.push(StoredTask {
            period: frequency.reduced().recip(),
            lazy: true,
            task: Box::new(callback),
        });

        self
    }
}
