use multiemu_rom::{RomId, RomManager, RomRequirement};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentVersion, SaveError},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
        WriteMemoryRecord,
    },
    platform::Platform,
};
use rand::RngCore;
use rangemap::RangeInclusiveMap;
use std::{
    borrow::Cow,
    io::{BufReader, BufWriter, Read, Write},
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

const PAGE_SIZE: usize = 4096;

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
    pub assigned_address_space: AddressSpaceHandle,
    pub initial_contents: RangeInclusiveMap<usize, StandardMemoryInitialContents>,
    pub sram: bool,
}

#[derive(Debug)]
pub struct StandardMemory {
    rom_manager: Arc<RomManager>,
    config: StandardMemoryConfig,
    buffer: Vec<Mutex<[u8; PAGE_SIZE]>>,
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
        reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, 0);

        let mut file = BufReader::new(reader);

        let assigned_start = *self.config.assigned_range.start();
        let assigned_end = *self.config.assigned_range.end();

        for (i, chunk) in self.buffer.iter().enumerate() {
            let start_addr = assigned_start + i * PAGE_SIZE;
            if start_addr > assigned_end {
                break;
            }

            let max_len = (assigned_end - start_addr + 1) as usize;
            let len = max_len.min(PAGE_SIZE);

            let mut locked_chunk = chunk.lock().unwrap();
            file.read_exact(&mut locked_chunk[..len])?;
        }

        Ok(())
    }

    fn store_save(&self, writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        assert!(self.config.sram, "Misbehaving save manager");

        // It's the exact same
        self.store_snapshot(writer)
    }

    fn store_snapshot(&self, writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = BufWriter::new(writer);
        let assigned_start = *self.config.assigned_range.start();
        let assigned_end = *self.config.assigned_range.end();

        for (i, chunk) in self.buffer.iter().enumerate() {
            let start_addr = assigned_start + i * PAGE_SIZE;
            if start_addr > assigned_end {
                break;
            }

            let max_len = (assigned_end - start_addr + 1) as usize;
            let len = max_len.min(PAGE_SIZE);

            let locked_chunk = chunk.lock().unwrap();
            file.write_all(&locked_chunk[..len])?;
        }

        Ok(())
    }

    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_before_range,
                    ReadMemoryRecord::Denied,
                )])
                .into());
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_after_range,
                    ReadMemoryRecord::Denied,
                )])
                .into());
            }
        }

        self.read_internal(address, buffer);

        Ok(())
    }

    fn preview_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_before_range,
                    PreviewMemoryRecord::Denied,
                )])
                .into());
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_after_range,
                    PreviewMemoryRecord::Denied,
                )])
                .into());
            }
        }

        self.read_internal(address, buffer);

        Ok(())
    }

    fn write_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_before_range,
                    WriteMemoryRecord::Denied,
                )])
                .into());
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_after_range,
                    WriteMemoryRecord::Denied,
                )])
                .into());
            }
        }

        // Shoved off in a helper function to prevent duplicated logic
        self.write_internal(address, buffer);

        Ok(())
    }
}

impl<P: Platform> ComponentConfig<P> for StandardMemoryConfig {
    type Component = StandardMemory;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        if self.assigned_range.is_empty() {
            return Err(BuildError::InvalidConfig(
                "Memory assigned must be non-empty".into(),
            ));
        }

        let rom_manager = component_builder.rom_manager();

        let buffer_size = self.assigned_range.clone().count();
        let chunks_needed = buffer_size.div_ceil(PAGE_SIZE);
        let buffer =
            Vec::from_iter(std::iter::repeat_n([0; PAGE_SIZE], chunks_needed).map(Mutex::new));
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

        let component_builder = match (self.readable, self.writable) {
            (true, true) => {
                component_builder.map_memory([(assigned_address_space, assigned_range)])
            }
            (true, false) => {
                component_builder.map_memory_read([(assigned_address_space, assigned_range)])
            }
            (false, true) => {
                component_builder.map_memory_write([(assigned_address_space, assigned_range)])
            }
            (false, false) => component_builder,
        };

        component_builder.build(component);

        Ok(())
    }
}

impl StandardMemory {
    /// Writes unchecked internally
    fn write_internal(&self, address: Address, buffer: &[u8]) {
        let requested_range = address - self.config.assigned_range.start()
            ..=(address - self.config.assigned_range.start() + buffer.len() - 1);

        let start_chunk = requested_range.start() / PAGE_SIZE;
        let end_chunk = requested_range.end() / PAGE_SIZE;

        let mut buffer_offset = 0;

        for chunk_index in start_chunk..=end_chunk {
            let chunk = &self.buffer[chunk_index];

            let chunk_start = if chunk_index == start_chunk {
                requested_range.start() % PAGE_SIZE
            } else {
                0
            };

            let chunk_end = if chunk_index == end_chunk {
                requested_range.end() % PAGE_SIZE
            } else {
                PAGE_SIZE - 1
            };

            // Lock the chunk and read the relevant part
            let mut locked_chunk = chunk.lock().unwrap();
            let chunk_range = chunk_start..=chunk_end;
            let buffer_range = buffer_offset..=buffer_offset + chunk_end - chunk_start;

            locked_chunk[chunk_range].copy_from_slice(&buffer[buffer_range]);

            buffer_offset += chunk_end - chunk_start + 1;

            if buffer_offset > buffer.len() {
                break;
            }
        }
    }

    fn read_internal(&self, address: Address, buffer: &mut [u8]) {
        let requested_range = address - self.config.assigned_range.start()
            ..=(address - self.config.assigned_range.start() + buffer.len() - 1);

        let start_chunk = requested_range.start() / PAGE_SIZE;
        let end_chunk = requested_range.end() / PAGE_SIZE;

        let mut buffer_offset = 0;

        for chunk_index in start_chunk..=end_chunk {
            let chunk = &self.buffer[chunk_index];

            let chunk_start = if chunk_index == start_chunk {
                requested_range.start() % PAGE_SIZE
            } else {
                0
            };

            let chunk_end = if chunk_index == end_chunk {
                requested_range.end() % PAGE_SIZE
            } else {
                PAGE_SIZE - 1
            };

            // Lock the chunk and read the relevant part
            let locked_chunk = chunk.lock().unwrap();
            let chunk_range = chunk_start..=chunk_end;
            let buffer_range = buffer_offset..=buffer_offset + chunk_end - chunk_start;

            buffer[buffer_range].copy_from_slice(&locked_chunk[chunk_range]);

            buffer_offset += chunk_end - chunk_start + 1;

            if buffer_offset > buffer.len() {
                break;
            }
        }
    }

    fn initialize_buffer(&self) {
        let internal_buffer_size = self.config.assigned_range.clone().count();

        // HACK: This overfills the buffer for ease of programming, but its ok because the actual mmu doesn't allow accesses out at runtime
        for (range, operation) in self.config.initial_contents.iter() {
            match operation {
                StandardMemoryInitialContents::Value(value) => {
                    let contents = vec![*value; range.clone().count()];
                    self.write_internal(*range.start(), &contents);
                }
                StandardMemoryInitialContents::Random => {
                    let mut contents = vec![0; range.clone().count()];
                    rand::rng().fill_bytes(contents.as_mut_slice());
                    self.write_internal(*range.start(), &contents);
                }
                StandardMemoryInitialContents::Array(value) => {
                    self.write_internal(*range.start(), value);
                }
                StandardMemoryInitialContents::Rom(rom_id) => {
                    let mut rom_file = self
                        .rom_manager
                        .open(*rom_id, RomRequirement::Required)
                        .unwrap();

                    let mut total_read = 0;
                    let mut buffer = [0; 4096];

                    while total_read < internal_buffer_size {
                        let remaining_space = internal_buffer_size - total_read;
                        let amount_to_read = remaining_space.min(buffer.len());
                        let amount = rom_file
                            .read(&mut buffer[..amount_to_read])
                            .expect("Could not read rom");

                        if amount == 0 {
                            break;
                        }

                        total_read += amount;

                        let write_size = remaining_space.min(amount);
                        self.write_internal(
                            *range.start() + total_read - amount,
                            &buffer[..write_size],
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use multiemu_runtime::{builder::MachineBuilder, utils::set_main_thread};

    #[test]
    fn initialization() {
        set_main_thread();

        let (machine, cpu_address_space) =
            MachineBuilder::new_test_minimal().insert_address_space(64);

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
        let machine = machine.build(Default::default());

        let mut buffer = [0; 4];

        machine
            .memory_access_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);

        let (machine, cpu_address_space) =
            MachineBuilder::new_test_minimal().insert_address_space(64);

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
        let machine = machine.build(Default::default());

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

        let (machine, cpu_address_space) =
            MachineBuilder::new_test_minimal().insert_address_space(64);

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
        let machine = machine.build(Default::default());

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

        let (machine, cpu_address_space) =
            MachineBuilder::new_test_minimal().insert_address_space(64);

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
        let machine = machine.build(Default::default());

        let buffer = [0; 8];

        machine
            .memory_access_table
            .write(0, cpu_address_space, &buffer)
            .unwrap();
    }

    #[test]
    fn basic_read_write() {
        set_main_thread();

        let (machine, cpu_address_space) =
            MachineBuilder::new_test_minimal().insert_address_space(64);

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
        let machine = machine.build(Default::default());

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

        let (machine, cpu_address_space) =
            MachineBuilder::new_test_minimal().insert_address_space(64);

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
        let machine = machine.build(Default::default());

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
