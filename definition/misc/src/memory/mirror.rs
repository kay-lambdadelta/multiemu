use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
    memory::{
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
};
use rangemap::RangeInclusiveMap;
use std::{collections::HashMap, ops::RangeInclusive};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PermissionSpace {
    Read,
    Write,
}

#[allow(clippy::type_complexity)]
#[derive(Debug, Default)]
pub struct MirrorMemoryConfig {
    assigned_ranges: HashMap<
        PermissionSpace,
        HashMap<AddressSpaceHandle, HashMap<RangeInclusive<usize>, (AddressSpaceHandle, usize)>>,
    >,
}

impl MirrorMemoryConfig {
    pub fn insert_range(
        mut self,
        source_addresses: RangeInclusive<usize>,
        source_address_space: AddressSpaceHandle,
        destination_addresses: RangeInclusive<usize>,
        destination_address_space: AddressSpaceHandle,
        permissions: impl IntoIterator<Item = PermissionSpace>,
    ) -> Self {
        assert_eq!(
            source_addresses.clone().count(),
            destination_addresses.clone().count(),
            "Addresses do not actually represent the same space"
        );

        for permission in permissions {
            let map = self
                .assigned_ranges
                .entry(permission)
                .or_default()
                .entry(source_address_space)
                .or_default();

            map.insert(
                source_addresses.clone(),
                (destination_address_space, *destination_addresses.start()),
            );
        }

        self
    }
}

#[derive(Debug)]
pub struct MirrorMemory;

impl Component for MirrorMemory {}

impl<R: RenderApi> ComponentConfig<R> for MirrorMemoryConfig {
    type Component = MirrorMemory;

    fn build_component(self, mut component_builder: ComponentBuilder<R, Self::Component>) {
        // TODO: A bit complex
        for (
            permission,
            source_address_space,
            source_addresses,
            destination_address_space,
            destination_address,
        ) in self
            .assigned_ranges
            .into_iter()
            .flat_map(|(permission, map)| {
                map.into_iter()
                    .flat_map(move |(source_address_space, map)| {
                        map.into_iter().map(
                            move |(
                                source_addresses,
                                (destination_address_space, destination_address),
                            )| {
                                (
                                    permission,
                                    source_address_space,
                                    source_addresses,
                                    destination_address_space,
                                    destination_address,
                                )
                            },
                        )
                    })
            })
        {
            let (builder, _) = match permission {
                PermissionSpace::Read => component_builder.insert_read_memory(
                    MirrorMemoryCallbacks {
                        source_addresses: source_addresses.clone(),
                        destination_address,
                        destination_address_space,
                    },
                    [(source_address_space, source_addresses)],
                ),
                PermissionSpace::Write => component_builder.insert_write_memory(
                    MirrorMemoryCallbacks {
                        source_addresses: source_addresses.clone(),
                        destination_address,
                        destination_address_space,
                    },
                    [(source_address_space, source_addresses)],
                ),
            };

            component_builder = builder;
        }

        component_builder.build_global(MirrorMemory);
    }
}

#[derive(Debug)]
struct MirrorMemoryCallbacks {
    source_addresses: RangeInclusive<usize>,
    destination_address: usize,
    destination_address_space: AddressSpaceHandle,
}

impl ReadMemory for MirrorMemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address =
            self.destination_address + (address - self.source_addresses.start());

        Err(RangeInclusiveMap::from_iter([(
            affected_range,
            ReadMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.destination_address_space,
            },
        )])
        .into())
    }

    fn preview_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address =
            self.destination_address + (address - self.source_addresses.start());

        Err(RangeInclusiveMap::from_iter([(
            affected_range,
            PreviewMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.destination_address_space,
            },
        )])
        .into())
    }
}

impl WriteMemory for MirrorMemoryCallbacks {
    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address =
            self.destination_address + (address - self.source_addresses.start());

        Err(RangeInclusiveMap::from_iter([(
            affected_range,
            WriteMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.destination_address_space,
            },
        )])
        .into())
    }
}

#[cfg(test)]
mod test {
    use crate::memory::{
        mirror::{MirrorMemoryConfig, PermissionSpace},
        standard::{StandardMemoryConfig, StandardMemoryInitialContents},
    };
    use multiemu_config::Environment;
    use multiemu_machine::{
        builder::MachineBuilder,
        display::{backend::software::SoftwareRendering, shader::ShaderCache},
    };
    use multiemu_rom::{manager::RomManager, system::GameSystem};
    use rangemap::RangeInclusiveMap;
    use std::{
        borrow::Cow,
        sync::{Arc, RwLock},
    };

    #[test]
    fn basic_read() {
        multiemu_machine::utils::set_main_thread();

        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::new(environment.clone());
        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager,
            environment,
            shader_cache,
        )
        .insert_address_space(64);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0..=7,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=7,
                    StandardMemoryInitialContents::Array(Cow::Owned(
                        (0..=7).map(|i| i as u8).collect(),
                    )),
                )]),
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror",
            MirrorMemoryConfig::default().insert_range(
                8..=15,
                cpu_address_space,
                0..=7,
                cpu_address_space,
                [PermissionSpace::Read, PermissionSpace::Write],
            ),
        );

        let machine = machine.build(Default::default());

        let mut buffer = [0; 8];

        machine
            .memory_translation_table
            .read(8, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn basic_write() {
        multiemu_machine::utils::set_main_thread();

        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::new(environment.clone());

        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager,
            environment,
            shader_cache,
        )
        .insert_address_space(64);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0..=7,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=7,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror",
            MirrorMemoryConfig::default().insert_range(
                8..=15,
                cpu_address_space,
                0..=7,
                cpu_address_space,
                [PermissionSpace::Read, PermissionSpace::Write],
            ),
        );

        let machine = machine.build(Default::default());

        let buffer = [0; 8];

        machine
            .memory_translation_table
            .write(8, cpu_address_space, &buffer)
            .unwrap();
    }

    #[test]
    fn extensive_read_test() {
        multiemu_machine::utils::set_main_thread();

        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::new(environment.clone());
        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager,
            environment,
            shader_cache,
        )
        .insert_address_space(64);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0..=3,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=3,
                    StandardMemoryInitialContents::Array(Cow::Owned(
                        (0..=3).map(|i| i as u8).collect(),
                    )),
                )]),
            },
        );

        let (machine, _) = machine.insert_component(
            "workram2",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 4..=7,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    4..=7,
                    StandardMemoryInitialContents::Array(Cow::Owned(
                        (4..=7).map(|i| i as u8).collect(),
                    )),
                )]),
            },
        );

        let (machine, _) = machine.insert_component(
            "workram-mirror",
            MirrorMemoryConfig::default().insert_range(
                8..=15,
                cpu_address_space,
                0..=7,
                cpu_address_space,
                [PermissionSpace::Read, PermissionSpace::Write],
            ),
        );
        let machine = machine.build(Default::default());

        let mut buffer = [0u8; 8];

        for (i, b) in buffer.iter_mut().enumerate() {
            *b = machine
                .memory_translation_table
                .read_le_value(i + 8, cpu_address_space)
                .unwrap();
        }

        assert_eq!(buffer, [0, 1, 2, 3, 4, 5, 6, 7]);
    }
}
