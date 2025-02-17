use crate::{
    component::{store::ComponentStore, Component, ComponentId, FromConfig},
    display::RenderBackend,
    memory::{memory_translation_table::MemoryTranslationTable, AddressSpaceId},
    scheduler::Scheduler,
    Machine,
};
use display::DisplayMetadata;
use input::InputMetadata;
use memory::MemoryMetadata;
use multiemu_config::Environment;
use multiemu_rom::{manager::RomManager, system::GameSystem};
use std::{
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
    rom_manager: Arc<RomManager>,
    game_system: GameSystem,
    component_store: ComponentStore,
    current_component_id: ComponentId,
    environment: Arc<RwLock<Environment>>,
    pub component_metadata: HashMap<ComponentId, ComponentMetadata>,
    memory_busses: HashMap<AddressSpaceId, u8>,
}

impl MachineBuilder {
    pub(crate) fn new(
        game_system: GameSystem,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
    ) -> Self {
        MachineBuilder {
            rom_manager,
            game_system,
            component_store: ComponentStore::default(),
            current_component_id: ComponentId(0),
            component_metadata: HashMap::new(),
            environment,
            memory_busses: HashMap::new(),
        }
    }

    pub fn insert_component<C: FromConfig>(mut self, config: C::Config) -> (Self, ComponentId) {
        let component_id = ComponentId(self.current_component_id.0);

        let component_builder = ComponentBuilder::<C> {
            machine_builder: &mut self,
            component_id,
            component_metadata: ComponentMetadata::default(),
            _phantom: PhantomData,
        };
        C::from_config(component_builder, config);

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
        display_component_initialization_data: Rc<R::ComponentInitializationData>,
    ) -> Machine {
        let component_store = Arc::new(self.component_store);
        let mut memory_translation_table = MemoryTranslationTable::default();

        for (address_space_id, width) in self.memory_busses.drain() {
            memory_translation_table.insert_bus(address_space_id, width);
        }

        for as_memory in self
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
            for (address_space, (assigned_ranges, callback)) in as_memory.read.drain() {
                memory_translation_table.insert_read_callback(
                    address_space,
                    assigned_ranges,
                    callback.clone(),
                );
            }

            for (address_space, (assigned_ranges, callback)) in as_memory.write.drain() {
                memory_translation_table.insert_write_callback(
                    address_space,
                    assigned_ranges,
                    callback.clone(),
                );
            }

            for (address_space, (assigned_ranges, callback)) in as_memory.preview.drain() {
                memory_translation_table.insert_preview_callback(
                    address_space,
                    assigned_ranges,
                    callback.clone(),
                );
            }
        }

        let memory_translation_table = Arc::new(memory_translation_table);
        Machine {
            memory_translation_table,
            scheduler: Scheduler::new(component_store.clone(), []),
            component_store,
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
    pub fn rom_manager(&self) -> Arc<RomManager> {
        self.machine_builder.rom_manager.clone()
    }

    pub fn environment(&self) -> Arc<RwLock<Environment>> {
        self.machine_builder.environment.clone()
    }

    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    pub fn build(self, component: C) {
        self.machine_builder
            .component_store
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
            .component_store
            .insert_component_global(self.component_id, component);

        self.machine_builder
            .component_metadata
            .insert(self.component_id, self.component_metadata);
    }
}
