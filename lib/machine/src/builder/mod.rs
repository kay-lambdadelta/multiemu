use crate::{
    component::{Component, ComponentId, FromConfig, RuntimeEssentials},
    display::RenderBackend,
    memory::{memory_translation_table::MemoryTranslationTable, AddressSpaceId},
    scheduler::Scheduler,
    Machine,
};
use display::{BackendSpecificData, DisplayMetadata};
use input::InputMetadata;
use memory::MemoryMetadata;
use multiemu_config::Environment;
use multiemu_rom::manager::RomManager;
use std::{
    any::TypeId,
    collections::HashMap,
    sync::{Arc, RwLock},
};
use std::{marker::PhantomData, rc::Rc};
use task::TaskMetadata;

pub mod display;
pub mod input;
pub mod memory;
pub mod task;

#[derive(Default)]
pub struct ComponentMetadata {
    pub memory: Option<MemoryMetadata>,
    pub task: Option<TaskMetadata>,
    pub display: Option<DisplayMetadata>,
    pub input: Option<InputMetadata>,
}

pub struct MachineBuilder {
    essentials: Arc<RuntimeEssentials>,
    current_component_id: ComponentId,
    pub component_metadata: HashMap<ComponentId, ComponentMetadata>,
    memory_busses: HashMap<AddressSpaceId, u8>,
}

impl MachineBuilder {
    pub fn new(rom_manager: Arc<RomManager>, environment: Arc<RwLock<Environment>>) -> Self {
        MachineBuilder {
            current_component_id: ComponentId(0),
            component_metadata: HashMap::new(),
            memory_busses: HashMap::new(),
            essentials: Arc::new(RuntimeEssentials::new(rom_manager, environment)),
        }
    }

    pub fn insert_component<C: FromConfig>(mut self, config: C::Config) -> (Self, ComponentId) {
        let component_id = ComponentId(self.current_component_id.0);

        let essentials = self.essentials.clone();
        let component_builder = ComponentBuilder::<C> {
            machine_builder: &mut self,
            component_id,
            component_metadata: ComponentMetadata::default(),
            _phantom: PhantomData,
        };
        C::from_config(component_builder, essentials, config);

        self.current_component_id.0 = self
            .current_component_id
            .0
            .checked_add(1)
            .expect("Too many components");

        (self, component_id)
    }

    pub fn insert_bus(mut self, address_space_id: AddressSpaceId, width: u8) -> Self {
        self.memory_busses.insert(address_space_id, width);
        self
    }

    pub fn insert_default_component<C: FromConfig>(self) -> (Self, ComponentId)
    where
        C::Config: Default,
    {
        let config = C::Config::default();
        self.insert_component::<C>(config)
    }

    pub fn build<R: RenderBackend>(
        mut self,
        display_component_initialization_data: Arc<R::ComponentInitializationData>,
    ) -> Machine<R> {
        let mut memory_translation_table = MemoryTranslationTable::default();

        for (address_space_id, width) in self.memory_busses.drain() {
            memory_translation_table.insert_bus(address_space_id, width);
        }

        for memory_metadata in self
            .component_metadata
            .iter_mut()
            .filter_map(|(_, metadata)| {
                if let Some(as_memory) = &mut metadata.memory {
                    Some(as_memory)
                } else {
                    None
                }
            })
        {
            for (address_space, (assigned_ranges, callback)) in memory_metadata.read.drain() {
                memory_translation_table.insert_read_callback(
                    address_space,
                    assigned_ranges,
                    callback.clone(),
                );
            }

            for (address_space, (assigned_ranges, callback)) in memory_metadata.write.drain() {
                memory_translation_table.insert_write_callback(
                    address_space,
                    assigned_ranges,
                    callback.clone(),
                );
            }

            for (address_space, (assigned_ranges, callback)) in memory_metadata.preview.drain() {
                memory_translation_table.insert_preview_callback(
                    address_space,
                    assigned_ranges,
                    callback.clone(),
                );
            }
        }

        let mut component_framebuffers = HashMap::new();
        let mut tasks = Vec::new();

        for (component_id, component_metadata) in self.component_metadata.drain() {
            if let Some(mut display_metadata) = component_metadata.display {
                self.essentials
                    .component_store()
                    .interact_dyn_local(component_id, |component| {
                        let (frame_sender, frame_receiver) = crossbeam::channel::bounded(1);

                        (display_metadata
                            .backend_specific_data
                            .remove(&TypeId::of::<R>())
                            .and_then(|item| item.downcast::<BackendSpecificData<R>>().ok())
                            .expect("Component did not register display backend")
                            .set_display_callback)(
                            component,
                            display_component_initialization_data.clone(),
                            frame_sender,
                        );

                        component_framebuffers.insert(component_id, frame_receiver);
                    });
            }

            if let Some(task_metadata) = component_metadata.task {
                tasks.push((component_id, task_metadata.frequency, task_metadata.task));
            }
        }

        let memory_translation_table = Arc::new(memory_translation_table);
        self.essentials
            .set_memory_translation_table(memory_translation_table.clone());

        let scheduler = Scheduler::new(self.essentials.component_store().clone(), tasks);

        Machine {
            scheduler,
            component_store: self.essentials.component_store().clone(),
            memory_translation_table,
            component_framebuffers,
        }
    }
}

pub struct ComponentBuilder<'a, C: Component> {
    machine_builder: &'a mut MachineBuilder,
    component_id: ComponentId,
    component_metadata: ComponentMetadata,
    _phantom: PhantomData<C>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    pub fn build(self, component: C) {
        self.machine_builder
            .essentials
            .component_store()
            .insert_component(self.component_id, component);

        self.machine_builder
            .component_metadata
            .insert(self.component_id, self.component_metadata);
    }

    /// Insert this component in the global store, ensuring quick access for all other components
    pub fn build_global(self, component: C)
    where
        C: Send + Sync,
    {
        self.machine_builder
            .essentials
            .component_store()
            .insert_component_global(self.component_id, component);

        self.machine_builder
            .component_metadata
            .insert(self.component_id, self.component_metadata);
    }
}
