use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{
        AddressSpaceId, VALID_MEMORY_ACCESS_SIZES,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use multiemu_rom::manager::RomManager;
use multiemu_rom::{id::RomId, manager::RomRequirement};
use rand::RngCore;
use rangemap::RangeInclusiveMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    io::Read,
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

const PAGE_SIZE: usize = 4096;

#[derive(Debug)]
pub enum StandardMemoryInitialContents {
    Value {
        value: u8,
    },
    Array {
        offset: usize,
        value: Cow<'static, [u8]>,
    },
    Rom {
        rom_id: RomId,
        offset: usize,
    },
    Random,
}

#[derive(Debug)]
pub struct StandardMemoryConfig {
    // If the buffer is readable
    pub readable: bool,
    // If the buffer is writable
    pub writable: bool,
    // The maximum word size
    pub max_word_size: usize,
    // Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<usize>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceId,
    // Initial contents
    pub initial_contents: Vec<StandardMemoryInitialContents>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StandardMemorySnapshot {
    pub memory: Vec<u8>,
}

#[derive(Debug)]
pub struct StandardMemory {
    memory_operation_callbacks: Arc<MemoryCallbacks>,
}

impl Component for StandardMemory {
    fn reset(&self) {
        self.memory_operation_callbacks.initialize_buffer();
    }
}

impl FromConfig for StandardMemory {
    type Config = StandardMemoryConfig;
    type Quirks = ();

    fn from_config(
        mut component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&config.max_word_size),
            "Invalid word size"
        );
        assert!(
            !config.assigned_range.is_empty(),
            "Memory assigned must be non-empty"
        );

        let buffer_size = config.assigned_range.clone().count();
        let chunks_needed = buffer_size.div_ceil(PAGE_SIZE);
        let buffer = Vec::from_iter(
            std::iter::repeat([0; PAGE_SIZE])
                .take(chunks_needed)
                .map(RwLock::new),
        );
        let assigned_range = config.assigned_range.clone();
        let assigned_address_space = config.assigned_address_space;

        let readable = config.readable;
        let writable = config.writable;
        let memory_operation_callbacks = Arc::new(MemoryCallbacks {
            config,
            buffer: buffer.into_iter().collect(),
            rom_manager: essentials.rom_manager().clone(),
        });
        memory_operation_callbacks.initialize_buffer();

        if readable {
            component_builder = component_builder.insert_read_memory(
                assigned_address_space,
                [assigned_range.clone()],
                memory_operation_callbacks.clone(),
            );
        }

        if writable {
            component_builder = component_builder.insert_write_memory(
                assigned_address_space,
                [assigned_range.clone()],
                memory_operation_callbacks.clone(),
            )
        }

        component_builder.build_global(Self {
            memory_operation_callbacks,
        });
    }
}

#[derive(Debug)]
struct MemoryCallbacks {
    config: StandardMemoryConfig,
    buffer: Vec<RwLock<[u8; PAGE_SIZE]>>,
    rom_manager: Arc<RomManager>,
}

impl ReadMemory for MemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        buffer: &mut [u8],
        _address_space: AddressSpaceId,
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                errors.insert(invalid_before_range, ReadMemoryRecord::Denied);
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                errors.insert(invalid_after_range, ReadMemoryRecord::Denied);
            }
        }

        if !errors.is_empty() {
            return;
        }

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
                PAGE_SIZE
            };

            // Lock the chunk and read the relevant part
            let locked_chunk = chunk.read().unwrap();
            buffer[buffer_offset..=buffer_offset + chunk_end - chunk_start]
                .copy_from_slice(&locked_chunk[chunk_start..=chunk_end]);

            buffer_offset += chunk_end - chunk_start;

            if buffer_offset >= buffer.len() {
                break;
            }
        }
    }
}

impl WriteMemory for MemoryCallbacks {
    fn write_memory(
        &self,
        address: usize,
        buffer: &[u8],
        _address_space: AddressSpaceId,
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        debug_assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                errors.insert(invalid_before_range, WriteMemoryRecord::Denied);
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                errors.insert(invalid_after_range, WriteMemoryRecord::Denied);
            }
        }

        if !errors.is_empty() {
            return;
        }

        // Shoved off in a helper function to prevent duplicated logic
        self.write_internal(address, buffer);
    }
}

impl MemoryCallbacks {
    /// Writes unchecked internally
    fn write_internal(&self, address: usize, buffer: &[u8]) {
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
                PAGE_SIZE
            };

            // Lock the chunk and read the relevant part
            let mut locked_chunk = chunk.write().unwrap();
            locked_chunk[chunk_start..=chunk_end]
                .copy_from_slice(&buffer[buffer_offset..=buffer_offset + chunk_end - chunk_start]);

            buffer_offset += chunk_end - chunk_start;

            if buffer_offset >= buffer.len() {
                break;
            }
        }
    }

    fn initialize_buffer(&self) {
        let internal_buffer_size = self.config.assigned_range.clone().count();

        // HACK: This overfills the buffer for ease of programming, but its ok because the actual mmu doesn't allow accesses out at runtime
        for operation in &self.config.initial_contents {
            match operation {
                StandardMemoryInitialContents::Value { value } => {
                    self.buffer
                        .par_iter()
                        .for_each(|chunk| chunk.write().unwrap().fill(*value));
                }
                StandardMemoryInitialContents::Random => {
                    self.buffer.par_iter().for_each(|chunk| {
                        rand::rng().fill_bytes(chunk.write().unwrap().as_mut_slice())
                    });
                }
                StandardMemoryInitialContents::Array { value, offset } => {
                    self.write_internal(*offset, value);
                }
                StandardMemoryInitialContents::Rom { rom_id, offset } => {
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
                        self.write_internal(*offset + total_read - amount, &buffer[..write_size]);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::RwLock;

    use multiemu_config::Environment;
    use multiemu_machine::{builder::MachineBuilder, display::software::SoftwareRendering};
    use multiemu_rom::system::GameSystem;

    use super::*;

    const ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

    #[test]
    fn initialization() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None).unwrap());
        let machine = MachineBuilder::new(
            GameSystem::Unknown,
            rom_manager.clone(),
            environment.clone(),
        )
        .insert_address_space(ADDRESS_SPACE, 64)
        .insert_component::<StandardMemory>(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=3,
                assigned_address_space: ADDRESS_SPACE,
                initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
            },
        )
        .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0; 4];

        machine
            .memory_translation_table()
            .read(0, ADDRESS_SPACE, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);

        let machine = MachineBuilder::new(GameSystem::Unknown, rom_manager.clone(), environment)
            .insert_address_space(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(
                "workram",
                StandardMemoryConfig {
                    max_word_size: 8,
                    readable: true,
                    writable: true,
                    assigned_range: 0..=3,
                    assigned_address_space: ADDRESS_SPACE,
                    initial_contents: vec![StandardMemoryInitialContents::Array {
                        value: Cow::Borrowed(&[0xff; 4]),
                        offset: 0,
                    }],
                },
            )
            .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0; 4];

        machine
            .memory_translation_table()
            .read(0, ADDRESS_SPACE, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);
    }

    #[test]
    fn basic_read() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None).unwrap());
        let machine = MachineBuilder::new(GameSystem::Unknown, rom_manager, environment)
            .insert_address_space(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(
                "workram",
                StandardMemoryConfig {
                    max_word_size: 8,
                    readable: true,
                    writable: true,
                    assigned_range: 0..=7,
                    assigned_address_space: ADDRESS_SPACE,
                    initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
                },
            )
            .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0; 8];

        machine
            .memory_translation_table()
            .read(0, ADDRESS_SPACE, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
    }

    #[test]
    fn basic_write() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None).unwrap());
        let machine = MachineBuilder::new(GameSystem::Unknown, rom_manager, environment)
            .insert_address_space(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(
                "workram",
                StandardMemoryConfig {
                    max_word_size: 8,
                    readable: true,
                    writable: true,
                    assigned_range: 0..=7,
                    assigned_address_space: ADDRESS_SPACE,
                    initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
                },
            )
            .build::<SoftwareRendering>(Default::default());
        let buffer = [0; 8];

        machine
            .memory_translation_table()
            .write(0, ADDRESS_SPACE, &buffer)
            .unwrap();
    }

    #[test]
    fn basic_read_write() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None).unwrap());
        let machine = MachineBuilder::new(GameSystem::Unknown, rom_manager, environment)
            .insert_address_space(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(
                "workram",
                StandardMemoryConfig {
                    max_word_size: 8,
                    readable: true,
                    writable: true,
                    assigned_range: 0..=7,
                    assigned_address_space: ADDRESS_SPACE,
                    initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
                },
            )
            .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0xff; 8];

        machine
            .memory_translation_table()
            .write(0, ADDRESS_SPACE, &buffer)
            .unwrap();
        buffer.fill(0);
        machine
            .memory_translation_table()
            .read(0, ADDRESS_SPACE, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
    }

    #[test]
    fn extensive() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None).unwrap());
        let machine = MachineBuilder::new(GameSystem::Unknown, rom_manager, environment)
            .insert_address_space(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(
                "workram",
                StandardMemoryConfig {
                    max_word_size: 8,
                    readable: true,
                    writable: true,
                    assigned_range: 0..=0xffff,
                    assigned_address_space: ADDRESS_SPACE,
                    initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
                },
            )
            .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0xff; 1];

        for i in 0..=0xffff {
            machine
                .memory_translation_table()
                .write(i, ADDRESS_SPACE, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_translation_table()
                .read(i, ADDRESS_SPACE, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 1]);
        }
    }
}
