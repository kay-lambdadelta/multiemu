use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    memory::{
        Address,
        callbacks::{Memory, ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
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
pub struct MirrorMemory;

impl Component for MirrorMemory {}

impl<B: ComponentBuilder<Component = MirrorMemory>> ComponentConfig<B> for MirrorMemoryConfig {
    type Component = MirrorMemory;

    fn build_component(self, mut component_builder: B) -> B::BuildOutput {
        let callback = MirrorMemoryCallbacks {
            source_addresses: self.source_addresses.clone(),
            destination_address: *self.destination_addresses.start(),
            destination_address_space: self.destination_address_space,
        };

        match (self.readable, self.writable) {
            (true, true) => {
                component_builder = component_builder
                    .insert_memory(
                        callback,
                        [(self.source_address_space, self.source_addresses)],
                    )
                    .0;
            }
            (true, false) => {
                component_builder = component_builder
                    .insert_read_memory(
                        callback,
                        [(self.source_address_space, self.source_addresses)],
                    )
                    .0;
            }
            (false, true) => {
                component_builder = component_builder
                    .insert_write_memory(
                        callback,
                        [(self.source_address_space, self.source_addresses)],
                    )
                    .0;
            }
            (false, false) => unimplemented!(),
        }

        component_builder.build_global(MirrorMemory)
    }
}

#[derive(Debug)]
struct MirrorMemoryCallbacks {
    source_addresses: RangeInclusive<Address>,
    destination_address: Address,
    destination_address_space: AddressSpaceHandle,
}

impl Memory for MirrorMemoryCallbacks {}

impl ReadMemory for MirrorMemoryCallbacks {
    fn read_memory(
        &self,
        address: Address,
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
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address =
            self.destination_address + (address - self.source_addresses.start());

        Err(RangeInclusiveMap::from_iter(std::iter::once((
            affected_range,
            PreviewMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.destination_address_space,
            },
        )))
        .into())
    }
}

impl WriteMemory for MirrorMemoryCallbacks {
    fn write_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        let affected_range = address..=(address + (buffer.len() - 1));
        let adjusted_destination_address =
            self.destination_address + (address - self.source_addresses.start());

        Err(RangeInclusiveMap::from_iter(std::iter::once((
            affected_range,
            WriteMemoryRecord::Redirect {
                address: adjusted_destination_address,
                address_space: self.destination_address_space,
            },
        )))
        .into())
    }
}

#[cfg(test)]
mod test {
    use crate::memory::{
        mirror::MirrorMemoryConfig,
        standard::{StandardMemoryConfig, StandardMemoryInitialContents},
    };
    use multiemu_rom::{manager::RomManager, system::GameSystem};
    use multiemu_runtime::{
        builder::MachineBuilder, display::backend::software::SoftwareRendering,
    };
    use num::rational::Ratio;
    use rangemap::RangeInclusiveMap;
    use std::{borrow::Cow, sync::Arc};

    #[test]
    fn basic_read() {
        unsafe { multiemu_runtime::utils::force_set_main_thread() };

        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager,
            Ratio::from_integer(44100),
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
        unsafe { multiemu_runtime::utils::force_set_main_thread() };

        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());

        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager,
            Ratio::from_integer(44100),
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
        unsafe { multiemu_runtime::utils::force_set_main_thread() };

        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager,
            Ratio::from_integer(44100),
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
