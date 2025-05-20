use crate::{
    Machine,
    component::{
        Component, ComponentConfig, ComponentId, RuntimeEssentials, component_ref::ComponentRef,
        store::ComponentStore,
    },
    display::{
        RenderExtensions,
        backend::{ContextExtensionSpecification, RenderApi},
        shader::ShaderCache,
    },
    memory::{AddressSpaceHandle, memory_translation_table::MemoryTranslationTable},
    scheduler::Scheduler,
};
use audio::AudioMetadata;
use display::DisplayMetadata;
use input::InputMetadata;
use multiemu_config::Environment;
use multiemu_rom::{manager::RomManager, system::GameSystem};
use std::{
    collections::HashMap,
    marker::PhantomData,
    sync::{Arc, OnceLock, RwLock},
};
use task::TaskMetadata;

pub mod audio;
pub mod display;
pub mod input;
pub mod memory;
pub mod task;

#[derive(Default)]
/// Overall data extracted from components needed for machine initialization
pub struct ComponentMetadata<R: RenderApi> {
    pub task: Option<TaskMetadata>,
    pub display: Option<DisplayMetadata<R>>,
    pub input: Option<InputMetadata>,
    pub audio: Option<AudioMetadata>,
}

/// Builder to produce a machine, definition crates will want to use this
pub struct MachineBuilder<R: RenderApi> {
    essentials: Arc<RuntimeEssentials<R>>,
    component_store: Arc<ComponentStore>,
    current_component_id: ComponentId,
    component_metadata: HashMap<ComponentId, ComponentMetadata<R>>,
    game_system: GameSystem,
}

impl<R: RenderApi> MachineBuilder<R> {
    pub fn new(
        game_system: GameSystem,
        rom_manager: Arc<RomManager>,
        environment: Arc<RwLock<Environment>>,
        shader_cache: ShaderCache,
    ) -> Self {
        MachineBuilder {
            current_component_id: ComponentId(0),
            component_store: Arc::default(),
            component_metadata: HashMap::new(),
            essentials: Arc::new(RuntimeEssentials {
                rom_manager,
                environment,
                shader_cache,
                memory_translation_table: MemoryTranslationTable::default(),
                render_initialization_data: OnceLock::default(),
            }),
            game_system,
        }
    }

    /// Insert a component into the machine
    #[inline]
    pub fn insert_component<C: ComponentConfig<R>>(
        mut self,
        name: &'static str,
        config: C,
    ) -> (Self, ComponentRef<C::Component>) {
        assert!(
            name.chars().all(|c| !c.is_whitespace()),
            "Invalid manifest name"
        );

        let component_id = ComponentId(self.current_component_id.0);

        let component_builder = ComponentBuilder::<R, C::Component> {
            machine_builder: &mut self,
            component_id,
            component_metadata: ComponentMetadata::default(),
            name,
            _phantom: PhantomData,
        };
        config.build_component(component_builder);

        self.current_component_id.0 = self
            .current_component_id
            .0
            .checked_add(1)
            .expect("Too many components");

        let component_ref = self.component_store.get(name).unwrap();

        (self, component_ref)
    }

    /// Insert a component with a default config
    pub fn insert_default_component<C: ComponentConfig<R> + Default>(
        self,
        name: &'static str,
    ) -> (Self, ComponentRef<C::Component>) {
        let config = C::default();
        self.insert_component(name, config)
    }

    /// Insert the required information to construct a address space
    pub fn insert_address_space(self, name: &'static str, width: u8) -> (Self, AddressSpaceHandle) {
        let id = self
            .essentials
            .memory_translation_table
            .insert_address_space(name, width);

        (self, id)
    }

    pub fn render_extensions(&self) -> RenderExtensions<R> {
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

        RenderExtensions {
            required,
            preferred,
        }
    }

    /// Build the machine
    pub fn build(
        mut self,
        component_initialization_data: R::ComponentInitializationData,
    ) -> Machine<R> {
        let mut framebuffers = Vec::new();
        let mut tasks = Vec::new();
        let mut all_gamepads = Vec::default();

        // So components do not panic
        self.essentials
            .render_initialization_data
            .set(component_initialization_data)
            .unwrap();

        for (component_id, component_metadata) in self.component_metadata.drain() {
            if let Some(display_metadata) = component_metadata.display {
                // Initialize all the display components
                self.component_store
                    .interact_dyn_local(component_id, |component| {
                        // Call the display callback
                        let framebuffer = (display_metadata.set_display_callback)(component);
                        framebuffers.push(framebuffer);
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
        let scheduler = Scheduler::new(self.component_store.clone(), tasks);

        Machine {
            scheduler,
            memory_translation_table: self.essentials.memory_translation_table.clone(),
            framebuffers,
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
}

/// Struct passed into components for their initialization purposes
pub struct ComponentBuilder<'a, R: RenderApi, C: Component> {
    machine_builder: &'a mut MachineBuilder<R>,
    component_id: ComponentId,
    component_metadata: ComponentMetadata<R>,
    name: &'static str,
    _phantom: PhantomData<C>,
}

impl<R: RenderApi, C: Component> ComponentBuilder<'_, R, C> {
    pub fn essentials(&self) -> Arc<RuntimeEssentials<R>> {
        self.machine_builder.essentials.clone()
    }

    /// Insert this component in the main thread's store, slowing down interactions but ensuring thread safety
    pub fn build(self, component: C) {
        self.machine_builder.component_store.insert_component(
            self.name,
            self.component_id,
            component,
        );

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
            .component_store
            .insert_component_global(self.name, self.component_id, component);

        self.machine_builder
            .component_metadata
            .insert(self.component_id, self.component_metadata);
    }
}
