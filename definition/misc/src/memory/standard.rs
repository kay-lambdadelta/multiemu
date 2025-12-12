use std::{
    borrow::Cow,
    io::{Read, Write},
    ops::RangeInclusive,
    sync::Arc,
};

use multiemu_range::{ContiguousRange, RangeIntersection};
use multiemu_runtime::{
    component::{Component, ComponentConfig, ComponentVersion},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, MemoryError},
    platform::Platform,
    program::{ProgramManager, RomId, RomRequirement},
};
use rand::RngCore;
use rangemap::RangeInclusiveMap;

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
    program_manager: Arc<ProgramManager>,
    config: StandardMemoryConfig,
    buffer: Vec<u8>,
}

impl Component for StandardMemory {
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

    fn memory_read(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        _avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        let requested_range = address - self.config.assigned_range.start()
            ..=(address - self.config.assigned_range.start() + buffer.len() - 1);

        buffer.copy_from_slice(&self.buffer[requested_range]);

        Ok(())
    }

    fn memory_write(
        &mut self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryError> {
        let requested_range = address - self.config.assigned_range.start()
            ..=(address - self.config.assigned_range.start() + buffer.len() - 1);

        self.buffer[requested_range].copy_from_slice(buffer);

        Ok(())
    }
}

impl<P: Platform> ComponentConfig<P> for StandardMemoryConfig {
    type Component = StandardMemory;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        if self.assigned_range.is_empty() {
            return Err("Memory assigned must be non-empty".into());
        }

        let program_manager = component_builder.program_manager();

        let buffer_size = self.assigned_range.clone().count();
        let buffer = vec![0; buffer_size];
        let assigned_range = self.assigned_range.clone();
        let assigned_address_space = self.assigned_address_space;

        let mut component = StandardMemory {
            config: self.clone(),
            buffer,
            program_manager: program_manager.clone(),
        };

        match component_builder.save() {
            Some((save, 0)) if self.sram => {
                // snapshot and save format are the exact same
                component.load_snapshot(0, save).unwrap();
            }
            Some(_) => return Err("Invalid save version".into()),
            None => {
                component.initialize_buffer();
            }
        }

        match (self.readable, self.writable) {
            (true, true) => {
                component_builder.memory_map_component(assigned_address_space, assigned_range)
            }
            (true, false) => {
                component_builder.memory_map_component_read(assigned_address_space, assigned_range)
            }
            (false, true) => {
                component_builder.memory_map_component_write(assigned_address_space, assigned_range)
            }
            (false, false) => component_builder,
        };

        Ok(component)
    }
}

impl StandardMemory {
    fn initialize_buffer(&mut self) {
        // HACK: This overfills the buffer for ease of programming, but its ok because
        // the actual mmu doesn't allow accesses out at runtime
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
                    self.buffer[range.clone()].copy_from_slice(value);
                }
                StandardMemoryInitialContents::Rom(rom_id) => {
                    let rom = self
                        .program_manager
                        .open(*rom_id, RomRequirement::Required)
                        .unwrap();

                    let actual_buffer_range = range.intersection(
                        &RangeInclusive::from_start_and_length(*range.start(), rom.len()),
                    );
                    let rom_range = 0..=(actual_buffer_range.end() - actual_buffer_range.start());

                    self.buffer[actual_buffer_range].copy_from_slice(&rom[rom_range]);
                }
            }
        }
    }
}
