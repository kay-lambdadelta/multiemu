use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig},
    memory::{
        Address, AddressSpaceId, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
        WriteMemoryRecord,
    },
    platform::Platform,
};
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct MirrorMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    pub source_addresses: RangeInclusive<Address>,
    pub source_address_space: AddressSpaceId,
    pub destination_addresses: RangeInclusive<Address>,
    pub destination_address_space: AddressSpaceId,
}

#[derive(Debug)]
pub struct MirrorMemory {
    pub config: MirrorMemoryConfig,
}

impl Component for MirrorMemory {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address = self.config.destination_addresses.start()
            + (address - self.config.source_addresses.start());

        Err(MemoryOperationError::from_iter([(
            affected_range,
            ReadMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.config.destination_address_space,
            },
        )]))
    }

    fn preview_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address = self.config.destination_addresses.start()
            + (address - self.config.source_addresses.start());

        Err(MemoryOperationError::from_iter(std::iter::once((
            affected_range,
            PreviewMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.config.destination_address_space,
            },
        ))))
    }

    fn write_memory(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address = self.config.destination_addresses.start()
            + (address - self.config.source_addresses.start());

        Err(MemoryOperationError::from_iter(std::iter::once((
            affected_range,
            WriteMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.config.destination_address_space,
            },
        ))))
    }
}

impl<P: Platform> ComponentConfig<P> for MirrorMemoryConfig {
    type Component = MirrorMemory;

    fn build_component(
        self,
        mut component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        if self.source_addresses.clone().count() != self.destination_addresses.clone().count() {
            return Err(BuildError::InvalidConfig(
                "Source and destination ranges must be the same length".into(),
            ));
        }

        if self.source_addresses.is_empty() {
            return Err(BuildError::InvalidConfig(
                "Memory assigned must be non-empty".into(),
            ));
        }

        match (self.readable, self.writable) {
            (true, true) => {
                component_builder = component_builder
                    .memory_map(self.source_address_space, self.source_addresses.clone());
            }
            (true, false) => {
                component_builder = component_builder
                    .memory_map_read(self.source_address_space, self.source_addresses.clone());
            }
            (false, true) => {
                component_builder = component_builder
                    .memory_map_write(self.source_address_space, self.source_addresses.clone());
            }
            (false, false) => {}
        }

        component_builder.build(MirrorMemory { config: self });

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::memory::{
        mirror::MirrorMemoryConfig,
        standard::{StandardMemoryConfig, StandardMemoryInitialContents},
    };
    use multiemu_runtime::{Machine, utils::set_main_thread};
    use rangemap::RangeInclusiveMap;
    use std::borrow::Cow;

    #[test]
    fn basic_read() {
        set_main_thread();

        let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

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
                sram: false,
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
            .memory_access_table
            .read(8, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0, 1, 2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn basic_write() {
        set_main_thread();

        let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

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
                sram: false,
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
            .memory_access_table
            .write(8, cpu_address_space, &buffer)
            .unwrap();
    }

    #[test]
    fn extensive_read_test() {
        set_main_thread();

        let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

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
                sram: false,
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
                sram: false,
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
                .memory_access_table
                .read_le_value(i + 8, cpu_address_space)
                .unwrap();
        }

        assert_eq!(buffer, [0, 1, 2, 3, 4, 5, 6, 7]);
    }
}
