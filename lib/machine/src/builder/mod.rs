use crate::{
    Machine,
    component::{Component, ComponentId, FromConfig, RuntimeEssentials, store::ComponentStore},
    display::{backend::RenderBackend, shader::ShaderCache},
    memory::{AddressSpaceHandle, memory_translation_table::MemoryTranslationTable},
    scheduler::Scheduler,
};
use display::{BackendSpecificData, DisplayMetadata};
use input::InputMetadata;
use multiemu_config::Environment;
use multiemu_rom::{manager::RomManager, system::GameSystem};
use std::{
    any::TypeId,
    collections::HashMap,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, RwLock},
};
use task::TaskMetadata;

pub mod display;
pub mod input;
pub mod memory;
pub mod task;

#[derive(Default)]
/// Overall data extracted from components needed for machine initialization
pub struct ComponentMetadata {
    pub task: Option<TaskMetadata>,
    pub display: Option<DisplayMetadata>,
    pub input: Option<InputMetadata>,
}

/// Builder to produce a machine, definition crates will want to use this
pub struct MachineBuilder {
    essentials: Arc<RuntimeEssentials>,
    current_component_id: ComponentId,
    pub component_metadata: HashMap<ComponentId, ComponentMetadata>,
    game_system: GameSystem,
}

impl MachineBuilder {
    pub fn new(
        game_system: GameSystem,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
        shader_cache: ShaderCache,
    ) -> Self {
        MachineBuilder {
            current_component_id: ComponentId(0),
            component_metadata: HashMap::new(),
            essentials: Arc::new(RuntimeEssentials {
                component_store: Arc::default(),
                rom_manager,
                environment,
                shader_cache,
                memory_translation_table: MemoryTranslationTable::default(),
            }),
            game_system,
        }
    }

    /// Insert a component into the machine
    pub fn insert_component<C: FromConfig>(
        mut self,
        manifest_name: &'static str,
        config: C::Config,
    ) -> Self {
        assert!(
            manifest_name.chars().all(|c| !c.is_whitespace()),
            "Invalid manifest name"
        );

        let component_id = ComponentId(self.current_component_id.0);

        let essentials = self.essentials.clone();
        let component_builder = ComponentBuilder::<C> {
            machine_builder: &mut self,
            component_id,
            component_metadata: ComponentMetadata::default(),
            manifest_name,
            _phantom: PhantomData,
        };
        C::from_config(component_builder, essentials, config, C::Quirks::default());

        self.current_component_id.0 = self
            .current_component_id
            .0
            .checked_add(1)
            .expect("Too many components");

        self
    }

    /// Insert a component with a default config
    pub fn insert_default_component<C: FromConfig>(self, manifest_name: &'static str) -> Self
    where
        C::Config: Default,
    {
        let config = C::Config::default();
        self.insert_component::<C>(manifest_name, config)
    }

    /// Insert the required information to construct a address space
    pub fn insert_address_space(self, name: &'static str, width: u8) -> (AddressSpaceHandle, Self) {
        let id = self
            .essentials
            .memory_translation_table
            .insert_address_space(name, width);

        (id, self)
    }

    /// Build the machine
    pub fn build<R: RenderBackend>(
        mut self,
        display_component_initialization_data: Rc<R::ComponentInitializationData>,
    ) -> Machine {
        let mut framebuffers = HashMap::new();
        let mut tasks = Vec::new();
        let mut all_gamepads = Vec::default();

        for (component_id, component_metadata) in self.component_metadata.drain() {
            if let Some(mut display_metadata) = component_metadata.display {
                // Initialize all the display components
                self.essentials
                    .component_store
                    .interact_dyn_local(component_id, |component| {
                        // Call the display callback
                        let framebuffer = (display_metadata
                            .backend_specific_data
                            .remove(&TypeId::of::<R>())
                            .and_then(|item| item.downcast::<BackendSpecificData<R>>().ok())
                            .expect("Component did not register display backend")
                            .set_display_callback)(
                            component,
                            display_component_initialization_data.clone(),
                        );

                        framebuffers.insert(component_id, framebuffer);
                    })
                    .unwrap();
            }

            // Gather the tasks
            if let Some(task_metadata) = component_metadata.task {
                tasks.extend(
                    task_metadata
                        .tasks
                        .into_iter()
                        .map(|(frequency, callback)| (component_id, frequency, callback)),
                );
            }

            if let Some(input_metadata) = component_metadata.input {
                let mut environment_guard = self.essentials.environment.write().unwrap();

                for gamepad in input_metadata.gamepads {
                    // Update the environment with default gamepad bounds
                    environment_guard
                        .gamepad_configs
                        .entry(self.game_system)
                        .or_default()
                        .entry(gamepad.name())
                        .or_insert_with(|| {
                            gamepad
                                .metadata()
                                .default_bindings
                                .clone()
                                .into_iter()
                                .collect()
                        });
                    all_gamepads.push(gamepad);
                }
            }
        }

        // Create the scheduler
        let scheduler = Scheduler::new(self.essentials.component_store.clone(), tasks);

        Machine {
            scheduler,
            component_store: self.essentials.component_store.clone(),
            memory_translation_table: self.essentials.memory_translation_table.clone(),
            framebuffers: Box::new(framebuffers),
            virtual_gamepads: all_gamepads
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
        }
    }

    pub fn component_store(&self) -> &ComponentStore {
        &self.essentials.component_store
    }
}

/// Struct passed into components for their initialization purposes
pub struct ComponentBuilder<'a, C: Component> {
    machine_builder: &'a mut MachineBuilder,
    component_id: ComponentId,
    component_metadata: ComponentMetadata,
    manifest_name: &'static str,
    _phantom: PhantomData<C>,
}

impl<C: Component> ComponentBuilder<'_, C> {
    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    pub fn build(self, component: C) {
        self.machine_builder
            .essentials
            .component_store
            .insert_component(self.manifest_name, self.component_id, component);

        self.machine_builder
            .component_metadata
            .insert(self.component_id, self.component_metadata);
    }

    /// Insert this component in the global store, ensuring quick access for all other components
    ///
    /// Use this if unsure
    pub fn build_global(self, component: C)
    where
        C: Send + Sync,
    {
        self.machine_builder
            .essentials
            .component_store
            .insert_component_global(self.manifest_name, self.component_id, component);

        self.machine_builder
            .component_metadata
            .insert(self.component_id, self.component_metadata);
    }
}
