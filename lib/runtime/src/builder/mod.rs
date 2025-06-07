use crate::{
    Machine,
    audio::AudioCallback,
    component::{
        Component, ComponentConfig, ComponentId, RuntimeEssentials, component_ref::ComponentRef,
        store::ComponentStore,
    },
    graphics::GraphicsCallback,
    input::virtual_gamepad::VirtualGamepad,
    memory::{
        Address,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{MemoryHandle, address_space::AddressSpaceHandle},
    },
    scheduler::{Scheduler, Task},
    utils::Fragile,
};
use audio::AudioMetadata;
use graphics::DisplayMetadata;
use input::InputMetadata;
use multiemu_audio::Sample;
use multiemu_graphics::{
    ContextExtensionSpecification, GraphicsApi, GraphicsContextExtensions, Software,
};
use multiemu_rom::{manager::RomManager, system::GameSystem};
use multiemu_save::ComponentName;
use num::rational::Ratio;
use rangemap::RangeInclusiveSet;
use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::RangeInclusive,
    sync::{Arc, OnceLock},
    vec::Vec,
};
use task::TaskMetadata;

pub mod audio;
pub mod graphics;
pub mod input;
pub mod memory;
pub mod task;

/// Overall data extracted from components needed for machine initialization
pub struct ComponentMetadata<R: GraphicsApi, S: Sample> {
    pub task: Option<TaskMetadata>,
    pub display: Option<DisplayMetadata<R>>,
    pub input: Option<InputMetadata>,
    pub audio: Option<AudioMetadata<S>>,
}

impl<R: GraphicsApi, S: Sample> Default for ComponentMetadata<R, S> {
    fn default() -> Self {
        Self {
            task: None,
            display: None,
            input: None,
            audio: None,
        }
    }
}

/// Builder to produce a machine, definition crates will want to use this
pub struct MachineBuilder<R: GraphicsApi = Software, S: Sample = f32> {
    essentials: Arc<RuntimeEssentials<R>>,
    component_store: Arc<ComponentStore>,
    current_component_id: ComponentId,
    component_metadata: HashMap<ComponentId, ComponentMetadata<R, S>>,
    game_system: GameSystem,
}

impl<R: GraphicsApi, S: Sample> MachineBuilder<R, S> {
    pub fn new(
        game_system: GameSystem,
        rom_manager: Arc<RomManager>,
        sample_rate: Ratio<u32>,
    ) -> Self {
        MachineBuilder::<R, S> {
            current_component_id: ComponentId(0),
            component_store: Arc::default(),
            component_metadata: HashMap::new(),
            essentials: Arc::new(RuntimeEssentials {
                rom_manager,
                memory_translation_table: Arc::default(),
                component_graphics_initialization_data: OnceLock::default(),
                sample_rate,
            }),
            game_system,
        }
    }

    pub fn new_test(rom_manager: Arc<RomManager>) -> Self {
        Self::new(
            GameSystem::Unknown,
            rom_manager,
            Ratio::from_integer(441000),
        )
    }

    /// Insert a component into the machine
    #[inline]
    pub fn insert_component<
        C: Component,
        B: ComponentConfig<ComponentBuilderImpl<R, S, C>, Component = C>,
    >(
        mut self,
        name: &str,
        config: B,
    ) -> (Self, ComponentRef<C>) {
        let name: ComponentName = name.parse().unwrap();

        let component_id = ComponentId(self.current_component_id.0);
        self.current_component_id.0 = self
            .current_component_id
            .0
            .checked_add(1)
            .expect("Too many components");
        let component_ref = ComponentRef::new(self.component_store.clone(), component_id);

        let component_builder = ComponentBuilderImpl {
            machine_builder: self,
            component_id,
            component_metadata: ComponentMetadata::default(),
            name: name.clone(),
            _phantom: PhantomData,
        };
        let me = config.build_component(component_ref.clone(), component_builder);

        (me, component_ref)
    }

    /// Insert a component with a default config
    pub fn insert_default_component<
        C: Component,
        B: ComponentConfig<ComponentBuilderImpl<R, S, C>, Component = C> + Default,
    >(
        self,
        name: &str,
    ) -> (Self, ComponentRef<C>) {
        let config = B::default();
        self.insert_component(name, config)
    }

    /// Insert the required information to construct a address space
    pub fn insert_address_space(self, width: u8) -> (Self, AddressSpaceHandle) {
        let id = self
            .essentials
            .memory_translation_table
            .insert_address_space(width);

        (self, id)
    }

    pub fn render_extensions(&self) -> GraphicsContextExtensions<R> {
        let preferred = self
            .component_metadata
            .iter()
            .filter_map(|(_, metadata)| {
                metadata
                    .display
                    .as_ref()
                    .and_then(|display| display.preferred_extensions.as_ref())
            })
            .fold(R::ContextExtensionSpecification::default(), |a, b| {
                a.combine(b.clone())
            });

        let required = self
            .component_metadata
            .iter()
            .filter_map(|(_, metadata)| {
                metadata
                    .display
                    .as_ref()
                    .and_then(|display| display.required_extensions.as_ref())
            })
            .fold(R::ContextExtensionSpecification::default(), |a, b| {
                a.combine(b.clone())
            });

        GraphicsContextExtensions {
            required,
            preferred,
        }
    }

    /// Build the machine
    pub fn build(
        mut self,
        component_initialization_data: R::ComponentGraphicsInitializationData,
    ) -> Machine {
        let mut tasks = Vec::new();
        let mut virtual_gamepads = Vec::default();
        let mut audio_callbacks = Vec::default();
        let mut graphics_callbacks = Vec::default();

        // So components do not panic
        self.essentials
            .component_graphics_initialization_data
            .set(component_initialization_data)
            .unwrap();

        for (_component_id, component_metadata) in self.component_metadata.drain() {
            // Gather the framebuffers
            if let Some(display_metadata) = component_metadata.display {
                graphics_callbacks.push(display_metadata.callback);
            }

            // Gather the tasks
            if let Some(task_metadata) = component_metadata.task {
                tasks.extend(task_metadata.tasks);
            }

            if let Some(input_metadata) = component_metadata.input {
                virtual_gamepads.extend(input_metadata.gamepads);
            }

            if let Some(audio_metadata) = component_metadata.audio {
                audio_callbacks.extend(audio_metadata.audio_data_callbacks);
            }
        }

        // Create the scheduler
        let scheduler = Scheduler::new(tasks, self.component_store.main_thread_queue.clone());

        // Make sure all the components do their proper post initialization
        self.component_store.interact_all(|compoenent| {
            compoenent.on_runtime_ready();
        });

        Machine {
            scheduler,
            memory_translation_table: self.essentials.memory_translation_table.clone(),
            virtual_gamepads: virtual_gamepads
                .into_iter()
                .enumerate()
                .map(|(index, gamepad)| {
                    (
                        index
                            .try_into()
                            .expect("How do you have this many gamepads"),
                        gamepad,
                    )
                })
                .collect(),
            game_system: self.game_system,
            audio_callbacks: Box::new(audio_callbacks),
            graphics_callbacks: Fragile::new(Box::new(graphics_callbacks)),
            _component_store: self.component_store,
        }
    }
}

#[doc(hidden)]
/// Struct passed into components for their initialization purposes. Do not refer to this directly.
pub struct ComponentBuilderImpl<R: GraphicsApi, S: Sample, C: Component> {
    machine_builder: MachineBuilder<R, S>,
    component_id: ComponentId,
    component_metadata: ComponentMetadata<R, S>,
    name: ComponentName,
    _phantom: PhantomData<C>,
}

#[sealed::sealed]
pub trait ComponentBuilder: Sized {
    /// Render api to use
    type GraphicsApi: GraphicsApi;
    /// Sample format to use
    type SampleFormat: Sample;
    /// Component to use
    type Component: Component;
    /// Build output
    type BuildOutput;

    /// Get the runtime essentials
    fn essentials(&self) -> Arc<RuntimeEssentials<Self::GraphicsApi>>;

    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    fn build(self, component: Self::Component) -> Self::BuildOutput;

    /// Insert this component in the global store, ensuring quick access for all other components
    ///
    /// Use this if unsure
    fn build_global(self, component: Self::Component) -> Self::BuildOutput
    where
        Self::Component: Send + Sync;

    fn insert_audio_callback(self, callback: impl AudioCallback<Self::SampleFormat>) -> Self;

    fn insert_screen(
        self,
        preferred_extensions: Option<
            <Self::GraphicsApi as GraphicsApi>::ContextExtensionSpecification,
        >,
        required_extensions: Option<
            <Self::GraphicsApi as GraphicsApi>::ContextExtensionSpecification,
        >,
        callback: impl GraphicsCallback<Self::GraphicsApi>,
    ) -> Self;

    fn insert_gamepad(self, gamepads: Arc<VirtualGamepad>) -> Self;

    /// Insert a callback into the memory translation table for reading
    fn insert_read_memory<M: ReadMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle);

    fn insert_write_memory<M: WriteMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle);

    fn insert_memory<M: ReadMemory + WriteMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle);

    fn insert_task(self, frequency: Ratio<u32>, callback: impl Task) -> Self;
}

#[sealed::sealed]
impl<R: GraphicsApi, S: Sample, C: Component> ComponentBuilder for ComponentBuilderImpl<R, S, C> {
    type GraphicsApi = R;
    type SampleFormat = S;
    type Component = C;
    type BuildOutput = MachineBuilder<R, S>;

    fn essentials(&self) -> Arc<RuntimeEssentials<R>> {
        self.machine_builder.essentials.clone()
    }

    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    fn build(mut self, component: C) -> Self::BuildOutput {
        self.machine_builder.component_store.insert_component(
            self.name,
            self.component_id,
            component,
        );

        self.machine_builder
            .component_metadata
            .insert(self.component_id, self.component_metadata);

        self.machine_builder
    }

    /// Insert this component in the global store, ensuring quick access for all other components
    ///
    /// Use this if unsure
    fn build_global(mut self, component: C) -> Self::BuildOutput
    where
        C: Send + Sync,
    {
        self.machine_builder
            .component_store
            .insert_component_global(self.name, self.component_id, component);

        self.machine_builder
            .component_metadata
            .insert(self.component_id, self.component_metadata);

        self.machine_builder
    }

    fn insert_audio_callback(mut self, callback: impl AudioCallback<S>) -> Self {
        let audio_data_callback = Box::new(callback);
        self.component_metadata
            .audio
            .get_or_insert_default()
            .audio_data_callbacks
            .push(audio_data_callback);

        self
    }

    fn insert_screen(
        mut self,
        preferred_extensions: Option<R::ContextExtensionSpecification>,
        required_extensions: Option<R::ContextExtensionSpecification>,
        callback: impl GraphicsCallback<R>,
    ) -> Self {
        self.component_metadata.display = Some(DisplayMetadata {
            preferred_extensions,
            required_extensions,
            callback: Box::new(callback),
        });

        self
    }

    fn insert_gamepad(mut self, gamepad: Arc<VirtualGamepad>) -> Self {
        self.component_metadata
            .input
            .get_or_insert_default()
            .gamepads
            .push(gamepad);

        self
    }

    /// Insert a callback into the memory translation table for reading
    fn insert_read_memory<M: ReadMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_read_memory(callback);

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
                .essentials
                .memory_translation_table
                .remap_read_memory(memory_handle, address_space, address_range);
        }

        (self, memory_handle)
    }

    fn insert_write_memory<M: WriteMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_write_memory(callback);

        let mut merged_addresses: HashMap<_, RangeInclusiveSet<_>> = HashMap::new();

        for (address_space, assigned_addresses) in assigned_addresses {
            merged_addresses
                .entry(address_space)
                .or_default()
                .insert(assigned_addresses);
        }

        for (address_space, address_range) in merged_addresses {
            self.machine_builder
                .essentials
                .memory_translation_table
                .remap_write_memory(memory_handle, address_space, address_range);
        }

        (self, memory_handle)
    }

    fn insert_memory<M: ReadMemory + WriteMemory>(
        self,
        callback: M,
        assigned_addresses: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<Address>)>,
    ) -> (Self, MemoryHandle) {
        let memory_handle = self
            .machine_builder
            .essentials
            .memory_translation_table
            .insert_memory(callback);

        let mut merged_addresses: HashMap<_, RangeInclusiveSet<_>> = HashMap::new();

        for (address_space, assigned_addresses) in assigned_addresses {
            merged_addresses
                .entry(address_space)
                .or_default()
                .insert(assigned_addresses);
        }

        for (address_space, address_range) in merged_addresses {
            self.machine_builder
                .essentials
                .memory_translation_table
                .remap_memory(memory_handle, address_space, address_range);
        }

        (self, memory_handle)
    }

    fn insert_task(mut self, frequency: Ratio<u32>, callback: impl Task) -> Self {
        let task_metatada = self.component_metadata.task.get_or_insert_default();

        task_metatada.tasks.push((frequency, Box::new(callback)));

        self
    }
}
