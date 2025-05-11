use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{
        AddressSpaceHandle, VALID_MEMORY_ACCESS_SIZES,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryHandle, PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord,
        },
    },
};
use multiemu_rom::{id::RomId, manager::RomRequirement};
use rand::RngCore;
use rangemap::RangeInclusiveMap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    borrow::Cow,
    io::{Read, Write},
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};
use versions::SemVer;

const PAGE_SIZE: usize = 4096 - size_of::<RwLock<()>>();

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct StandardMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    // The maximum word size
    pub max_word_size: usize,
    // Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<usize>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceHandle,
    // Initial contents
    pub initial_contents: Vec<StandardMemoryInitialContents>,
}

pub struct StandardMemory {
    memory_operation_callbacks: Arc<StandardMemoryCallbacks>,
    pub memory_handle: MemoryHandle,
}

impl Component for StandardMemory {
    fn reset(&self) {
        self.memory_operation_callbacks.initialize_buffer();
    }

    fn load(
        &self,
        entry: &mut dyn Read,
        version: SemVer,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(version, SemVer::new("1.0.0").unwrap());

        for chunk in self.memory_operation_callbacks.buffer.iter() {
            let mut buffer = chunk.write().unwrap();
            entry.read_exact(buffer.as_mut_slice())?;
        }

        Ok(())
    }

    fn save(&self, entry: &mut dyn Write) -> Result<SemVer, Box<dyn std::error::Error>> {
        for chunk in self.memory_operation_callbacks.buffer.iter() {
            entry.write_all(chunk.read().unwrap().as_slice())?;
        }

        Ok(SemVer::new("1.0.0").unwrap())
    }
}

impl FromConfig for StandardMemory {
    type Config = StandardMemoryConfig;
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
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
        let buffer =
            Vec::from_iter(std::iter::repeat_n([0; PAGE_SIZE], chunks_needed).map(RwLock::new));
        let assigned_range = config.assigned_range.clone();
        let assigned_address_space = config.assigned_address_space;

        let memory_operation_callbacks = Arc::new(StandardMemoryCallbacks {
            config: config.clone(),
            buffer: buffer.into_iter().collect(),
            essentials: essentials.clone(),
        });
        memory_operation_callbacks.initialize_buffer();

        let memory_handle = match (config.readable, config.writable) {
            (true, true) => essentials.memory_translation_table.insert_memory(
                memory_operation_callbacks.clone(),
                [(assigned_address_space, assigned_range)],
            ),
            (true, false) => essentials.memory_translation_table.insert_read_memory(
                memory_operation_callbacks.clone(),
                [(assigned_address_space, assigned_range)],
            ),
            (false, true) => essentials.memory_translation_table.insert_write_memory(
                memory_operation_callbacks.clone(),
                [(assigned_address_space, assigned_range)],
            ),
            (false, false) => {
                panic!("What?");
            }
        };

        component_builder.build_global(Self {
            memory_operation_callbacks,
            memory_handle,
        });
    }
}

#[derive(Debug)]
struct StandardMemoryCallbacks {
    config: StandardMemoryConfig,
    buffer: Vec<RwLock<[u8; PAGE_SIZE]>>,
    essentials: Arc<RuntimeEssentials>,
}

impl ReadMemory for StandardMemoryCallbacks {
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

        self.read_internal(address, buffer);
    }

    fn preview_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, PreviewMemoryRecord>,
    ) {
        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                errors.insert(invalid_before_range, PreviewMemoryRecord::Denied);
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                errors.insert(invalid_after_range, PreviewMemoryRecord::Denied);
            }
        }

        if !errors.is_empty() {
            return;
        }

        self.read_internal(address, buffer);
    }
}

impl WriteMemory for StandardMemoryCallbacks {
    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
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

impl StandardMemoryCallbacks {
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
                PAGE_SIZE - 1
            };

            // Lock the chunk and read the relevant part
            let mut locked_chunk = chunk.write().unwrap();
            let chunk_range = chunk_start..=chunk_end;
            let buffer_range = buffer_offset..=buffer_offset + chunk_end - chunk_start;

            locked_chunk[chunk_range].copy_from_slice(&buffer[buffer_range]);

            buffer_offset += chunk_end - chunk_start + 1;

            if buffer_offset > buffer.len() {
                break;
            }
        }
    }

    fn read_internal(&self, address: usize, buffer: &mut [u8]) {
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
            let locked_chunk = chunk.read().unwrap();
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
                        .essentials
                        .rom_manager
                        .open(
                            *rom_id,
                            RomRequirement::Required,
                            &self.essentials.environment.read().unwrap().roms_directory,
                        )
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
    use multiemu_config::Environment;
    use multiemu_machine::{
        builder::MachineBuilder,
        display::{backend::software::SoftwareRendering, shader::ShaderCache},
    };
    use multiemu_rom::{manager::RomManager, system::GameSystem};
    use std::sync::RwLock;

    use super::*;

    #[test]
    fn initialization() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::default();

        let (machine, cpu_address_space) = MachineBuilder::new(
            GameSystem::Unknown,
            rom_manager.clone(),
            environment.clone(),
            shader_cache.clone(),
        )
        .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component::<StandardMemory>(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=3,
                assigned_address_space: cpu_address_space,
                initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
            },
        );
        let machine = machine.build::<SoftwareRendering>(Default::default());

        let mut buffer = [0; 4];

        machine
            .memory_translation_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);

        let (machine, cpu_address_space) = MachineBuilder::new(
            GameSystem::Unknown,
            rom_manager.clone(),
            environment.clone(),
            shader_cache.clone(),
        )
        .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component::<StandardMemory>(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=3,
                assigned_address_space: cpu_address_space,
                initial_contents: vec![StandardMemoryInitialContents::Array {
                    value: Cow::Borrowed(&[0xff; 4]),
                    offset: 0,
                }],
            },
        );
        let machine = machine.build::<SoftwareRendering>(Default::default());

        let mut buffer = [0; 4];

        machine
            .memory_translation_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);
    }

    #[test]
    fn basic_read() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::default();

        let (machine, cpu_address_space) =
            MachineBuilder::new(GameSystem::Unknown, rom_manager, environment, shader_cache)
                .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component::<StandardMemory>(
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
        let machine = machine.build::<SoftwareRendering>(Default::default());

        let mut buffer = [0; 8];

        machine
            .memory_translation_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
    }

    #[test]
    fn basic_write() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::default();

        let (machine, cpu_address_space) =
            MachineBuilder::new(GameSystem::Unknown, rom_manager, environment, shader_cache)
                .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component::<StandardMemory>(
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
        let machine = machine.build::<SoftwareRendering>(Default::default());

        let buffer = [0; 8];

        machine
            .memory_translation_table
            .write(0, cpu_address_space, &buffer)
            .unwrap();
    }

    #[test]
    fn basic_read_write() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::default();

        let (machine, cpu_address_space) =
            MachineBuilder::new(GameSystem::Unknown, rom_manager, environment, shader_cache)
                .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component::<StandardMemory>(
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
        let machine = machine.build::<SoftwareRendering>(Default::default());

        let mut buffer = [0xff; 8];

        machine
            .memory_translation_table
            .write(0, cpu_address_space, &buffer)
            .unwrap();
        buffer.fill(0);
        machine
            .memory_translation_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
    }

    #[test]
    fn extensive() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::default();

        let (machine, cpu_address_space) =
            MachineBuilder::new(GameSystem::Unknown, rom_manager, environment, shader_cache)
                .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component::<StandardMemory>(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=0xffff,
                assigned_address_space: cpu_address_space,
                initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
            },
        );
        let machine = machine.build::<SoftwareRendering>(Default::default());

        for i in 0..=0x5000 {
            let mut buffer = [0xff; 1];
            machine
                .memory_translation_table
                .write(i, cpu_address_space, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_translation_table
                .read(i, cpu_address_space, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 1]);

            let mut buffer = [0xff; 2];
            machine
                .memory_translation_table
                .write(i, cpu_address_space, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_translation_table
                .read(i, cpu_address_space, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 2]);

            let mut buffer = [0xff; 4];
            machine
                .memory_translation_table
                .write(i, cpu_address_space, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_translation_table
                .read(i, cpu_address_space, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 4]);

            let mut buffer = [0xff; 8];
            machine
                .memory_translation_table
                .write(i, cpu_address_space, &buffer)
                .unwrap();
            buffer.fill(0x00);
            machine
                .memory_translation_table
                .read(i, cpu_address_space, &mut buffer)
                .unwrap();
            assert_eq!(buffer, [0xff; 8]);
        }
    }
}
