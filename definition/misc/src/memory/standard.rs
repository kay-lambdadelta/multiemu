use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig},
    memory::{
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
        AddressSpaceId, VALID_MEMORY_ACCESS_SIZES,
    },
};
use multiemu_rom::manager::RomManager;
use multiemu_rom::{id::RomId, manager::RomRequirement};
use rand::RngCore;
use rangemap::RangeMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    io::Read,
    ops::Range,
    sync::{Arc, Mutex},
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
    pub assigned_range: Range<usize>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceId,
    // Initial contents
    pub initial_contents: StandardMemoryInitialContents,
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

    fn from_config(mut component_builder: ComponentBuilder<Self>, config: Self::Config) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&config.max_word_size),
            "Invalid word size"
        );
        assert!(
            !config.assigned_range.is_empty(),
            "Memory assigned must be non-empty"
        );

        let buffer_size = config.assigned_range.len();
        let chunks_needed = buffer_size.div_ceil(PAGE_SIZE);
        let buffer = Vec::from_iter(
            std::iter::repeat([0; PAGE_SIZE])
                .take(chunks_needed)
                .map(Mutex::new),
        );
        let assigned_range = config.assigned_range.clone();
        let assigned_address_space = config.assigned_address_space;

        let readable = config.readable;
        let writable = config.writable;
        let memory_operation_callbacks = Arc::new(MemoryCallbacks {
            config,
            buffer: buffer.into_iter().collect(),
            rom_manager: component_builder.rom_manager(),
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
    buffer: Vec<Mutex<[u8; PAGE_SIZE]>>,
    rom_manager: Arc<RomManager>,
}

impl ReadMemory for MemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        buffer: &mut [u8],
        _address_space: AddressSpaceId,
        errors: &mut RangeMap<usize, ReadMemoryRecord>,
    ) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        let requested_range = address - self.config.assigned_range.start
            ..address - self.config.assigned_range.start + buffer.len();
        let invalid_before_range = address..self.config.assigned_range.start;
        let invalid_after_range = self.config.assigned_range.end..address + buffer.len();

        if !invalid_after_range.is_empty() || !invalid_before_range.is_empty() {
            errors.extend(
                [invalid_after_range, invalid_before_range]
                    .into_iter()
                    .filter_map(|range| {
                        if !range.is_empty() {
                            Some((range, ReadMemoryRecord::Denied))
                        } else {
                            None
                        }
                    }),
            );
        }

        if !errors.is_empty() {
            return;
        }

        let start_chunk = requested_range.start / PAGE_SIZE;
        let end_chunk = requested_range.end.div_ceil(PAGE_SIZE);

        let mut buffer_offset = 0;

        for chunk_index in start_chunk..end_chunk {
            let chunk = &self.buffer[chunk_index];

            let chunk_start = if chunk_index == start_chunk {
                requested_range.start % PAGE_SIZE
            } else {
                0
            };

            let chunk_end = if chunk_index == end_chunk - 1 {
                // If we're in the last chunk, handle the exact range end
                if requested_range.end % PAGE_SIZE == 0 && requested_range.end != 0 {
                    PAGE_SIZE
                } else {
                    requested_range.end % PAGE_SIZE
                }
            } else {
                PAGE_SIZE
            };

            // Lock the chunk and read the relevant part
            let locked_chunk = chunk.lock().unwrap();
            buffer[buffer_offset..buffer_offset + chunk_end - chunk_start]
                .copy_from_slice(&locked_chunk[chunk_start..chunk_end]);

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
        errors: &mut RangeMap<usize, WriteMemoryRecord>,
    ) {
        debug_assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        let invalid_before_range = address..self.config.assigned_range.start;
        let invalid_after_range = self.config.assigned_range.end..address + buffer.len();

        if !invalid_after_range.is_empty() || !invalid_before_range.is_empty() {
            errors.extend(
                [invalid_after_range, invalid_before_range]
                    .into_iter()
                    .filter_map(|range| {
                        if !range.is_empty() {
                            Some((range, WriteMemoryRecord::Denied))
                        } else {
                            None
                        }
                    }),
            );
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
        let requested_range = address - self.config.assigned_range.start
            ..address - self.config.assigned_range.start + buffer.len();

        let start_chunk = requested_range.start / PAGE_SIZE;
        let end_chunk = requested_range.end.div_ceil(PAGE_SIZE);

        let mut buffer_offset = 0;

        for chunk_index in start_chunk..end_chunk {
            let chunk = &self.buffer[chunk_index];

            let chunk_start = if chunk_index == start_chunk {
                requested_range.start % PAGE_SIZE
            } else {
                0
            };

            let chunk_end = if chunk_index == end_chunk - 1 {
                // If we're in the last chunk, handle the exact range end
                if requested_range.end % PAGE_SIZE == 0 && requested_range.end != 0 {
                    PAGE_SIZE
                } else {
                    requested_range.end % PAGE_SIZE
                }
            } else {
                PAGE_SIZE
            };

            let mut locked_chunk = chunk.lock().unwrap();
            locked_chunk[chunk_start..chunk_end]
                .copy_from_slice(&buffer[buffer_offset..buffer_offset + chunk_end - chunk_start]);

            buffer_offset += chunk_end - chunk_start;

            if buffer_offset >= buffer.len() {
                break;
            }
        }
    }

    fn initialize_buffer(&self) {
        let internal_buffer_size = self.config.assigned_range.len();

        // HACK: This overfills the buffer for ease of programming, but its ok because the actual mmu doesn't allow accesses out at runtime
        match &self.config.initial_contents {
            StandardMemoryInitialContents::Value { value } => {
                self.buffer
                    .par_iter()
                    .for_each(|chunk| chunk.lock().unwrap().fill(*value));
            }
            StandardMemoryInitialContents::Random => {
                self.buffer
                    .par_iter()
                    .for_each(|chunk| rand::rng().fill_bytes(chunk.lock().unwrap().as_mut_slice()));
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

#[cfg(test)]
mod test {
    use std::sync::RwLock;

    use multiemu_config::Environment;
    use multiemu_machine::{display::software::SoftwareRendering, Machine};
    use multiemu_rom::system::GameSystem;

    use super::*;

    const ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

    #[test]
    fn initialization() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None).unwrap());
        let machine = Machine::build(
            GameSystem::Unknown,
            rom_manager.clone(),
            environment.clone(),
        )
        .insert_bus(ADDRESS_SPACE, 64)
        .insert_component::<StandardMemory>(StandardMemoryConfig {
            max_word_size: 8,
            readable: true,
            writable: true,
            assigned_range: 0..4,
            assigned_address_space: ADDRESS_SPACE,
            initial_contents: StandardMemoryInitialContents::Value { value: 0xff },
        })
        .0
        .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0; 4];

        machine
            .memory_translation_table()
            .read(0, ADDRESS_SPACE, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);

        let machine = Machine::build(GameSystem::Unknown, rom_manager.clone(), environment)
            .insert_bus(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..4,
                assigned_address_space: ADDRESS_SPACE,
                initial_contents: StandardMemoryInitialContents::Array {
                    value: Cow::Borrowed(&[0xff; 4]),
                    offset: 0,
                },
            })
            .0
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
        let machine = Machine::build(GameSystem::Unknown, rom_manager, environment)
            .insert_bus(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..0x10000,
                assigned_address_space: ADDRESS_SPACE,
                initial_contents: StandardMemoryInitialContents::Value { value: 0xff },
            })
            .0
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
        let machine = Machine::build(GameSystem::Unknown, rom_manager, environment)
            .insert_bus(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..0x10000,
                assigned_address_space: ADDRESS_SPACE,
                initial_contents: StandardMemoryInitialContents::Value { value: 0xff },
            })
            .0
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
        let machine = Machine::build(GameSystem::Unknown, rom_manager, environment)
            .insert_bus(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..0x10000,
                assigned_address_space: ADDRESS_SPACE,
                initial_contents: StandardMemoryInitialContents::Value { value: 0xff },
            })
            .0
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
        let machine = Machine::build(GameSystem::Unknown, rom_manager, environment)
            .insert_bus(ADDRESS_SPACE, 64)
            .insert_component::<StandardMemory>(StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..0x10000,
                assigned_address_space: ADDRESS_SPACE,
                initial_contents: StandardMemoryInitialContents::Value { value: 0xff },
            })
            .0
            .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0xff; 1];

        for i in 0..0x10000 {
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
