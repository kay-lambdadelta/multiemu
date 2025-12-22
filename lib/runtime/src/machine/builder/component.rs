use std::{
    any::Any,
    collections::{HashMap, HashSet},
    io::Read,
    marker::PhantomData,
    ops::RangeInclusive,
    sync::Arc,
};

use bytes::Bytes;

use crate::{
    component::{
        Component, ComponentConfig, ComponentHandle, ComponentVersion, LateInitializedData,
        TypedComponentHandle,
    },
    input::VirtualGamepad,
    machine::{
        builder::{MachineBuilder, PartialEvent, SchedulerParticipation},
        graphics::GraphicsRequirements,
    },
    memory::{
        Address, AddressSpace, AddressSpaceId, MapTarget, MemoryRemappingCommand, Permissions,
    },
    path::{FluxEmuPath, Namespace},
    platform::Platform,
    program::ProgramManager,
    scheduler::{EventType, Frequency, Period, PreemptionSignal},
};

/// Overall data extracted from components needed for machine initialization
pub(super) struct ComponentMetadata<P: Platform> {
    pub displays: HashSet<FluxEmuPath>,
    pub graphics_requirements: GraphicsRequirements<P::GraphicsApi>,
    pub audio_outputs: HashSet<FluxEmuPath>,
    pub gamepads: HashMap<FluxEmuPath, Arc<VirtualGamepad>>,
    #[allow(clippy::type_complexity)]
    pub late_initializer: Box<dyn FnOnce(&mut dyn Component, &LateInitializedData<P>)>,
    pub scheduler_participation: SchedulerParticipation,
    pub events: Vec<PartialEvent>,
    pub preemption_signal: Arc<PreemptionSignal>,
}

impl<P: Platform> ComponentMetadata<P> {
    pub fn new<B: ComponentConfig<P>>() -> Self {
        Self {
            displays: Default::default(),
            graphics_requirements: Default::default(),
            audio_outputs: Default::default(),
            gamepads: Default::default(),
            late_initializer: Box::new(|component, data| {
                let component: &mut B::Component =
                    (component as &mut dyn Any).downcast_mut().unwrap();
                B::late_initialize(component, data);
            }),
            scheduler_participation: SchedulerParticipation::None,
            events: Vec::default(),
            preemption_signal: Arc::default(),
        }
    }
}

pub struct ComponentBuilder<'a, P: Platform, C: Component> {
    pub(super) machine_builder: &'a mut MachineBuilder<P>,
    pub(super) component_metadata: &'a mut ComponentMetadata<P>,
    pub(super) path: &'a FluxEmuPath,
    pub(super) _phantom: PhantomData<C>,
}

impl<'a, P: Platform, C: Component> ComponentBuilder<'a, P, C> {
    pub fn path(&self) -> &'a FluxEmuPath {
        self.path
    }

    pub fn program_manager(&self) -> &Arc<ProgramManager> {
        self.machine_builder.program_manager()
    }

    pub fn get_address_space(&self, address_space: AddressSpaceId) -> &Arc<AddressSpace> {
        &self.machine_builder.address_spaces[&address_space].address_space
    }

    pub fn save(&self) -> Option<(Box<dyn Read>, ComponentVersion)> {
        None
    }

    pub fn set_scheduler_participation(
        self,
        scheduler_participation: SchedulerParticipation,
    ) -> Self {
        self.component_metadata.scheduler_participation = scheduler_participation;

        self
    }

    pub fn interact<C2: Component, T>(
        &self,
        path: &FluxEmuPath,
        callback: impl FnOnce(&C2) -> T,
    ) -> Option<T> {
        self.machine_builder
            .registry
            .interact_without_synchronization(path, callback)
    }

    pub fn interact_mut<C2: Component, T: 'static>(
        &self,
        path: &FluxEmuPath,
        callback: impl FnOnce(&mut C2) -> T,
    ) -> Option<T> {
        self.machine_builder
            .registry
            .interact_mut_without_synchronization(path, callback)
    }

    pub fn typed_handle<C2: Component>(
        &self,
        path: &FluxEmuPath,
    ) -> Option<TypedComponentHandle<C2>> {
        self.machine_builder.registry.typed_handle(path)
    }

    pub fn handle(&self, path: &FluxEmuPath) -> Option<ComponentHandle> {
        self.machine_builder.registry.handle(path)
    }

    /// Insert a component into the machine
    pub fn insert_child_component<B: ComponentConfig<P>>(
        self,
        name: &str,
        config: B,
    ) -> (Self, FluxEmuPath) {
        assert!(
            !name.contains(FluxEmuPath::SEPARATOR),
            "This function requires a name not a path"
        );

        let mut component_path = self.path.clone();
        component_path.push(Namespace::Component, name);

        let mut component_metadata = ComponentMetadata::new::<B>();

        let component_builder = ComponentBuilder::<P, B::Component> {
            machine_builder: self.machine_builder,
            component_metadata: &mut component_metadata,
            path: &component_path,
            _phantom: PhantomData,
        };

        let component = config
            .build_component(component_builder)
            .expect("Failed to build component");

        self.machine_builder.registry.insert_component(
            component_path.clone(),
            component_metadata.scheduler_participation,
            self.machine_builder.scheduler.event_queue.clone(),
            component_metadata.preemption_signal.clone(),
            component,
        );

        self.machine_builder
            .component_metadata
            .insert(component_path.clone(), component_metadata);

        (self, component_path)
    }

    /// Insert a component with a default config
    pub fn insert_default_child_component<B: ComponentConfig<P> + Default>(
        self,
        name: &str,
    ) -> (Self, FluxEmuPath) {
        let config = B::default();
        self.insert_child_component(name, config)
    }

    pub fn insert_audio_channel(self, name: &str) -> (Self, FluxEmuPath) {
        let mut resource_path = self.path.clone();
        resource_path.push(Namespace::Resource, name);

        self.component_metadata
            .audio_outputs
            .insert(resource_path.clone());

        (self, resource_path)
    }

    pub fn insert_display(self, name: &str) -> (Self, FluxEmuPath) {
        let mut resource_path = self.path.clone();
        resource_path.push(Namespace::Resource, name);

        self.component_metadata
            .displays
            .insert(resource_path.clone());

        (self, resource_path)
    }

    pub fn insert_gamepad(self, name: &str, gamepad: Arc<VirtualGamepad>) -> (Self, FluxEmuPath) {
        let mut resource_path = self.path.clone();
        resource_path.push(Namespace::Resource, name);

        self.component_metadata
            .gamepads
            .insert(resource_path.clone(), gamepad);

        (self, resource_path)
    }

    /// Insert a callback into the memory translation table for reading
    pub fn memory_map_component_read(
        self,
        address_space: AddressSpaceId,
        range: RangeInclusive<Address>,
    ) -> Self {
        self.machine_builder
            .address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range,
                target: MapTarget::Component(self.path.clone()),
                permissions: Permissions {
                    read: true,
                    write: false,
                },
            });

        self
    }

    pub fn memory_map_component_write(
        self,
        address_space: AddressSpaceId,
        range: RangeInclusive<Address>,
    ) -> Self {
        self.machine_builder
            .address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range,
                target: MapTarget::Component(self.path.clone()),
                permissions: Permissions {
                    read: false,
                    write: true,
                },
            });

        self
    }

    pub fn memory_map_component(
        self,
        address_space: AddressSpaceId,
        range: RangeInclusive<Address>,
    ) -> Self {
        self.machine_builder
            .address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range,
                target: MapTarget::Component(self.path.clone()),
                permissions: Permissions {
                    read: true,
                    write: true,
                },
            });

        self
    }

    pub fn memory_mirror_map_read(
        self,
        address_space: AddressSpaceId,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
    ) -> Self {
        self.machine_builder
            .address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range: source,
                target: MapTarget::Mirror { destination },
                permissions: Permissions {
                    read: true,
                    write: false,
                },
            });

        self
    }

    pub fn memory_mirror_map_write(
        self,
        address_space: AddressSpaceId,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
    ) -> Self {
        self.machine_builder
            .address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range: source,
                target: MapTarget::Mirror { destination },
                permissions: Permissions {
                    read: false,
                    write: true,
                },
            });

        self
    }

    pub fn memory_mirror_map(
        self,
        address_space: AddressSpaceId,
        source: RangeInclusive<Address>,
        destination: RangeInclusive<Address>,
    ) -> Self {
        self.machine_builder
            .address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range: source,
                target: MapTarget::Mirror { destination },
                permissions: Permissions {
                    read: true,
                    write: true,
                },
            });

        self
    }

    pub fn memory_register_buffer(
        self,
        address_space: AddressSpaceId,
        name: &str,
        buffer: Bytes,
    ) -> (Self, FluxEmuPath) {
        let mut resource_path = self.path.clone();
        resource_path.push(Namespace::Resource, name);

        self.machine_builder
            .address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Register {
                path: resource_path.clone(),
                buffer,
            });

        (self, resource_path)
    }

    pub fn memory_map_buffer_read(
        self,
        address_space: AddressSpaceId,
        range: RangeInclusive<Address>,
        path: &FluxEmuPath,
    ) -> Self {
        self.machine_builder
            .address_spaces
            .get_mut(&address_space)
            .unwrap()
            .memory_map_queue
            .push(MemoryRemappingCommand::Map {
                range,
                target: MapTarget::Memory(path.clone()),
                permissions: Permissions {
                    read: true,
                    write: false,
                },
            });

        self
    }

    pub fn schedule_event(
        self,
        time: Period,
        callback: impl FnOnce(&mut C, Period) + Send + Sync + 'static,
    ) -> Self {
        self.component_metadata.events.push(PartialEvent {
            ty: EventType::Once {
                callback: Box::new(move |component, timestamp| {
                    let component = (component as &mut dyn Any).downcast_mut().unwrap();

                    callback(component, timestamp);
                }),
            },
            time,
        });

        self
    }

    pub fn schedule_repeating_event(
        self,
        time: Period,
        frequency: Frequency,
        mut callback: impl FnMut(&mut C, Period) + Send + Sync + 'static,
    ) -> Self {
        self.component_metadata.events.push(PartialEvent {
            ty: EventType::Repeating {
                callback: Box::new(move |component, timestamp| {
                    let component = (component as &mut dyn Any).downcast_mut().unwrap();

                    callback(component, timestamp);
                }),
                frequency,
            },
            time,
        });

        self
    }
}
