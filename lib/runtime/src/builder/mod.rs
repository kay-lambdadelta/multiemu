use crate::{
    Machine,
    audio::{AudioCallback, AudioOutputId, AudioOutputInfo},
    component::{
        Component, ComponentConfig, ComponentId, ComponentRef, ComponentStore, RuntimeEssentials,
    },
    graphics::{DisplayCallback, DisplayId, DisplayInfo, GraphicsRequirements},
    input::{VirtualGamepad, VirtualGamepadId},
    memory::{Address, AddressSpaceHandle, MemoryTranslationTable},
    platform::{Platform, TestPlatform},
    scheduler::{Scheduler, Task},
    utils::{DirectMainThreadExecutor, MainThreadQueue},
};
use audio::AudioMetadata;
use graphics::GraphicsMetadata;
use input::InputMetadata;
use multiemu_graphics::GraphicsApi;
use multiemu_rom::RomManager;
use multiemu_save::ComponentName;
use num::rational::Ratio;
use pathfinding::prelude::topological_sort;
use rangemap::RangeInclusiveSet;
use rustc_hash::FxBuildHasher;
use std::{
    collections::{HashMap, HashSet},
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
    memory_translation_table: Arc<MemoryTranslationTable>,
    /// Rom manager
    rom_manager: Arc<RomManager>,
    /// Selected sample rate
    sample_rate: Ratio<u32>,
    /// The store for components
    component_store: Arc<ComponentStore>,
    /// Stored component builder callbacks for late initialization
    component_builders: HashMap<ComponentId, ComponentBuilderCallback<P>>,
    /// Graphics requirements
    graphics_requirements: GraphicsRequirements<P::GraphicsApi>,
    /// Component metadata
    component_metadata: HashMap<ComponentId, ComponentMetadata<P>, FxBuildHasher>,
    /// Counter for assigning things
    current_component_id: ComponentId,
    current_audio_output_id: AudioOutputId,
    current_display_id: DisplayId,
}

impl<P: Platform> MachineBuilder<P> {
    pub fn new(
        rom_manager: Arc<RomManager>,
        sample_rate: Ratio<u32>,
        main_thread_executor: Arc<P::MainThreadExecutor>,
    ) -> Self {
        let main_thread_queue = MainThreadQueue::new(main_thread_executor);
        let component_store = ComponentStore::new(main_thread_queue.clone());

        MachineBuilder::<P> {
            current_component_id: ComponentId(0),
            current_audio_output_id: AudioOutputId(0),
            current_display_id: DisplayId(0),
            component_builders: HashMap::default(),
            memory_translation_table: Arc::new(MemoryTranslationTable::new(
                component_store.clone(),
            )),
            component_store,
            rom_manager,
            sample_rate,
            graphics_requirements: GraphicsRequirements::default(),
            component_metadata: HashMap::default(),
        }
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

        let component_id = ComponentId(self.current_component_id.0);
        self.current_component_id.0 = self
            .current_component_id
            .0
            .checked_add(1)
            .expect("Too many components");
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
                let component_builder = ComponentBuilder::<P, B::Component> {
                    runtime_essentials,
                    machine_builder,
                    component_id,
                    name: name.clone(),
                    _phantom: PhantomData,
                };

                config.build_component(component_ref.clone(), component_builder);
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
        let component_ids: Vec<_> = (0..self.current_component_id.0)
            .map(|id| ComponentId(id))
            .collect();

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
            memory_translation_table: self.memory_translation_table.clone(),
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

        let mut tasks = Vec::new();
        let mut virtual_gamepads = Vec::default();
        let mut audio_outputs = HashMap::default();
        let mut displays = HashMap::default();

        for (_, component_metadata) in self.component_metadata.drain() {
            // Gather the framebuffers
            if let Some(graphics_metadata) = component_metadata.graphics {
                displays.extend(graphics_metadata.displays);
            }

            // Gather the tasks
            if let Some(task_metadata) = component_metadata.task {
                tasks.extend(task_metadata.tasks);
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

        // Make sure all the components do their proper post initialization
        self.component_store.interact_all(|compoenent| {
            compoenent.on_runtime_ready();
        });

        Machine {
            scheduler: Mutex::new(scheduler),
            memory_translation_table: self.memory_translation_table.clone(),
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
            component_store: self.component_store,
            displays,
            audio_outputs,
        }
    }
}

impl MachineBuilder<TestPlatform> {
    pub fn new_test(rom_manager: Arc<RomManager>) -> Self {
        Self::new(
            rom_manager,
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

    pub fn insert_task(mut self, frequency: Ratio<u32>, callback: impl Task) -> Self {
        let task_metatada = self.metadata().task.get_or_insert_default();

        task_metatada.tasks.push((frequency, Box::new(callback)));

        self
    }
}
