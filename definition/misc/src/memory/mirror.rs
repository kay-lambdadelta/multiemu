use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig, ComponentRef},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
        WriteMemoryRecord,
    },
    platform::Platform,
};
use rangemap::RangeInclusiveMap;
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct MirrorMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    pub source_addresses: RangeInclusive<Address>,
    pub source_address_space: AddressSpaceHandle,
    pub destination_addresses: RangeInclusive<Address>,
    pub destination_address_space: AddressSpaceHandle,
}

#[derive(Debug)]
pub struct MirrorMemory {
    pub config: MirrorMemoryConfig,
}

impl Component for MirrorMemory {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address = self.config.destination_addresses.start()
            + (address - self.config.source_addresses.start());

        Err(RangeInclusiveMap::from_iter([(
            affected_range,
            ReadMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.config.destination_address_space,
            },
        )])
        .into())
    }

    fn preview_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address = self.config.destination_addresses.start()
            + (address - self.config.source_addresses.start());

        Err(RangeInclusiveMap::from_iter(std::iter::once((
            affected_range,
            PreviewMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.config.destination_address_space,
            },
        )))
        .into())
    }

    fn write_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address = self.config.destination_addresses.start()
            + (address - self.config.source_addresses.start());

        Err(RangeInclusiveMap::from_iter(std::iter::once((
            affected_range,
            WriteMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.config.destination_address_space,
            },
        )))
        .into())
    }
}

impl<P: Platform> ComponentConfig<P> for MirrorMemoryConfig {
    type Component = MirrorMemory;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        mut component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) {
        assert_eq!(
            self.source_addresses.clone().count(),
            self.destination_addresses.clone().count(),
            "Source and destination ranges must be the same length"
        );

        match (self.readable, self.writable) {
            (true, true) => {
                component_builder = component_builder
                    .map_memory([(self.source_address_space, self.source_addresses.clone())]);
            }
            (true, false) => {
                component_builder = component_builder
                    .map_memory_read([(self.source_address_space, self.source_addresses.clone())]);
            }
            (false, true) => {
                component_builder = component_builder
                    .map_memory_write([(self.source_address_space, self.source_addresses.clone())]);
            }
            (false, false) => {}
        }

        component_builder.build_global(MirrorMemory { config: self })
    }
}

#[cfg(test)]
mod test {
    use crate::memory::{
        mirror::MirrorMemoryConfig,
        standard::{StandardMemoryConfig, StandardMemoryInitialContents},
    };
    use multiemu_rom::RomManager;
    use multiemu_runtime::{builder::MachineBuilder, utils::set_main_thread};
    use rangemap::RangeInclusiveMap;
    use std::{borrow::Cow, sync::Arc};

    #[test]
    fn basic_read() {
        set_main_thread();

        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let (machine, cpu_address_space) =
            MachineBuilder::new_test(rom_manager).insert_address_space(64);

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
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 8..=15,
                source_address_space: cpu_address_space,
                destination_addresses: 0..=7,
                destination_address_space: cpu_address_space,
            },
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
        set_main_thread();

        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());

        let (machine, cpu_address_space) =
            MachineBuilder::new_test(rom_manager).insert_address_space(64);

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
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 8..=15,
                source_address_space: cpu_address_space,
                destination_addresses: 0..=7,
                destination_address_space: cpu_address_space,
            },
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
        set_main_thread();

        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let (machine, cpu_address_space) =
            MachineBuilder::new_test(rom_manager).insert_address_space(64);

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
            MirrorMemoryConfig {
                readable: true,
                writable: true,
                source_addresses: 8..=15,
                source_address_space: cpu_address_space,
                destination_addresses: 0..=7,
                destination_address_space: cpu_address_space,
            },
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
