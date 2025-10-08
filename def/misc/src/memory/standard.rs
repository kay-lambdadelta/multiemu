use multiemu::{
    component::{BuildError, Component, ComponentConfig, ComponentVersion, SaveError},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, PreviewMemoryError, ReadMemoryError, WriteMemoryError},
    platform::Platform,
    rom::{RomId, RomMetadata, RomRequirement},
};
use rand::RngCore;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use std::{
    borrow::Cow,
    io::{Read, Write},
    ops::RangeInclusive,
    sync::Arc,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StandardMemoryInitialContents {
    Value(u8),
    Array(Cow<'static, [u8]>),
    Rom(RomId),
    Random,
}

#[derive(Debug, Clone)]
pub struct StandardMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    pub assigned_range: RangeInclusive<Address>,
    pub assigned_address_space: AddressSpaceId,
    pub initial_contents: RangeInclusiveMap<usize, StandardMemoryInitialContents>,
    pub sram: bool,
}

#[derive(Debug)]
pub struct StandardMemory {
    rom_manager: Arc<RomMetadata>,
    config: StandardMemoryConfig,
    buffer: Vec<u8>,
}

impl Component for StandardMemory {
    fn reset(&mut self) {
        self.initialize_buffer();
    }

    // The save/snapshot format is just raw bytes so i doubt it will ever change

    fn save_version(&self) -> Option<ComponentVersion> {
        if self.config.sram { Some(0) } else { None }
    }

    fn snapshot_version(&self) -> Option<ComponentVersion> {
        Some(0)
    }

    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        mut reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);

        reader.read_exact(&mut self.buffer)?;

        Ok(())
    }

    fn store_save(&self, writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        assert!(self.config.sram, "Misbehaving save manager");

        // It's the exact same
        self.store_snapshot(writer)
    }

    fn store_snapshot(&self, mut writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        writer.write_all(&self.buffer)?;

        Ok(())
    }

    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        let requested_range = address - self.config.assigned_range.start()
            ..=(address - self.config.assigned_range.start() + buffer.len() - 1);

        buffer.copy_from_slice(&self.buffer[requested_range]);

        Ok(())
    }

    fn preview_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), PreviewMemoryError> {
        let requested_range = address - self.config.assigned_range.start()
            ..=(address - self.config.assigned_range.start() + buffer.len() - 1);

        buffer.copy_from_slice(&self.buffer[requested_range]);

        Ok(())
    }

    fn write_memory(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        let requested_range = address - self.config.assigned_range.start()
            ..=(address - self.config.assigned_range.start() + buffer.len() - 1);

        self.buffer[requested_range].copy_from_slice(&buffer);

        Ok(())
    }
}

impl<P: Platform> ComponentConfig<P> for StandardMemoryConfig {
    type Component = StandardMemory;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, BuildError> {
        if self.assigned_range.is_empty() {
            return Err(BuildError::InvalidConfig(
                "Memory assigned must be non-empty".into(),
            ));
        }

        let rom_manager = component_builder.rom_manager();

        let buffer_size = self.assigned_range.clone().count();
        let buffer = vec![0; buffer_size];
        let assigned_range = self.assigned_range.clone();
        let assigned_address_space = self.assigned_address_space;

        let mut component = StandardMemory {
            config: self.clone(),
            buffer,
            rom_manager: rom_manager.clone(),
        };

        match component_builder.save() {
            Some((save, 0)) if self.sram => {
                // snapshot and save format are the exact same
                component.load_snapshot(0, save).unwrap();
            }
            Some(_) => return Err(BuildError::LoadingSave(SaveError::InvalidVersion)),
            None => {
                component.initialize_buffer();
            }
        }

        match (self.readable, self.writable) {
            (true, true) => component_builder.memory_map(assigned_address_space, assigned_range),
            (true, false) => {
                component_builder.memory_map_read(assigned_address_space, assigned_range)
            }
            (false, true) => {
                component_builder.memory_map_write(assigned_address_space, assigned_range)
            }
            (false, false) => component_builder,
        };

        Ok(component)
    }
}

impl StandardMemory {
    fn initialize_buffer(&mut self) {
        // HACK: This overfills the buffer for ease of programming, but its ok because the actual mmu doesn't allow accesses out at runtime
        for (range, operation) in self.config.initial_contents.iter() {
            let range = range.start() - self.config.assigned_range.start()
                ..=(range.end() - self.config.assigned_range.start());

            match operation {
                StandardMemoryInitialContents::Value(value) => {
                    self.buffer[range.clone()].fill(*value);
                }
                StandardMemoryInitialContents::Random => {
                    rand::rng().fill_bytes(&mut self.buffer[range.clone()]);
                }
                StandardMemoryInitialContents::Array(value) => {
                    self.buffer[range.clone()].copy_from_slice(&value);
                }
                StandardMemoryInitialContents::Rom(rom_id) => {
                    let mut rom_file = self
                        .rom_manager
                        .open(*rom_id, RomRequirement::Required)
                        .unwrap();

                    let mut rom = Vec::new();
                    rom_file.read_to_end(&mut rom).unwrap();
                    let actual_buffer_range: RangeInclusive<Address> = range
                        .clone()
                        .intersection(*range.start()..=(*range.start() + rom.len() - 1))
                        .into();
                    let rom_range = 0..=(actual_buffer_range.end() - actual_buffer_range.start());

                    self.buffer[actual_buffer_range].copy_from_slice(&rom[rom_range]);
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use multiemu::{machine::Machine, utils::set_main_thread};

    use super::*;

    #[test]
    fn initialization() {
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
                    StandardMemoryInitialContents::Value(0xff),
                )]),
                sram: false,
            },
        );
        let machine = machine.build(Default::default(), false);

        let mut buffer = [0; 4];

        machine
            .memory_access_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);

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
                    StandardMemoryInitialContents::Value(0xff),
                )]),
                sram: false,
            },
        );
        let machine = machine.build(Default::default(), false);

        let mut buffer = [0; 4];

        machine
            .memory_access_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);
    }

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
                    StandardMemoryInitialContents::Value(0xff),
                )]),
                sram: false,
            },
        );
        let machine = machine.build(Default::default(), false);

        let mut buffer = [0; 8];

        machine
            .memory_access_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
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
        let machine = machine.build(Default::default(), false);

        let buffer = [0; 8];

        machine
            .memory_access_table
            .write(0, cpu_address_space, &buffer)
            .unwrap();
    }

    #[test]
    fn basic_read_write() {
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
        let machine = machine.build(Default::default(), false);

        let mut buffer = [0xff; 8];

        machine
            .memory_access_table
            .write(0, cpu_address_space, &buffer)
            .unwrap();
        buffer.fill(0);
        machine
            .memory_access_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
    }

    #[test]
    fn extensive() {
        set_main_thread();

        let (machine, cpu_address_space) = Machine::build_test_minimal().insert_address_space(16);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                readable: true,
                writable: true,
                assigned_range: 0..=0xffff,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=0xffff,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
                sram: false,
            },
        );
        let machine = machine.build(Default::default(), false);

        for i in 0..=0x5000 {
            let mut buffer = [0xff; 1];
            machine
                .memory_access_table
                .write(i, cpu_address_space, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_access_table
                .read(i, cpu_address_space, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 1]);

            let mut buffer = [0xff; 2];
            machine
                .memory_access_table
                .write(i, cpu_address_space, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_access_table
                .read(i, cpu_address_space, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 2]);

            let mut buffer = [0xff; 4];
            machine
                .memory_access_table
                .write(i, cpu_address_space, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_access_table
                .read(i, cpu_address_space, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 4]);

            let mut buffer = [0xff; 8];
            machine
                .memory_access_table
                .write(i, cpu_address_space, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_access_table
                .read(i, cpu_address_space, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 8]);
        }
    }
}
