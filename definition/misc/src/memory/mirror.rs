use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
    memory::{
        AddressSpaceHandle, VALID_MEMORY_ACCESS_SIZES,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord},
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
        HashMap<AddressSpaceHandle, RangeInclusiveMap<usize, (AddressSpaceHandle, usize)>>,
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
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address =
            self.destination_address + (address - self.source_addresses.start());

        errors.insert(
            affected_range,
            ReadMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.destination_address_space,
            },
        );
    }

    fn preview_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, PreviewMemoryRecord>,
    ) {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address =
            self.destination_address + (address - self.source_addresses.start());

        errors.insert(
            affected_range,
            PreviewMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.destination_address_space,
            },
        );
    }
}

impl WriteMemory for MirrorMemoryCallbacks {
    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address =
            self.destination_address + (address - affected_range.start());

        errors.insert(
            affected_range,
            WriteMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.destination_address_space,
            },
        );
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
    use std::sync::{Arc, RwLock};

    #[test]
    fn basic_read() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::default();
        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager,
            environment,
            shader_cache,
        )
        .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=7,
                assigned_address_space: cpu_address_space,
                initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
            },
        );

        let (machine, _) = machine.insert_component(
            "workram-mirror",
            MirrorMemoryConfig::default().insert_range(
                0x10000..=0x1ffff,
                cpu_address_space,
                0x0000..=0xffff,
                cpu_address_space,
                [PermissionSpace::Read, PermissionSpace::Write],
            ),
        );
        let machine = machine.build(Default::default());

        let mut buffer = [0; 8];

        machine
            .memory_translation_table
            .read(0x10000, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
    }

    #[test]
    fn basic_write() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::default();

        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager,
            environment,
            shader_cache,
        )
        .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=7,
                assigned_address_space: cpu_address_space,
                initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
            },
        );
        let (machine, _) = machine.insert_component(
            "workram-mirror",
            MirrorMemoryConfig::default().insert_range(
                0x10000..=0x1ffff,
                cpu_address_space,
                0x0000..=0xffff,
                cpu_address_space,
                [PermissionSpace::Read, PermissionSpace::Write],
            ),
        );

        let machine = machine.build(Default::default());

        let buffer = [0; 8];

        machine
            .memory_translation_table
            .write(0x10000, cpu_address_space, &buffer)
            .unwrap();
    }
}
