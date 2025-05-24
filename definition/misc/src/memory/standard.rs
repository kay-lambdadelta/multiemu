use multiemu_config::Environment;
use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
    memory::{
        AddressSpaceHandle, VALID_MEMORY_ACCESS_SIZES,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{
            MemoryHandle, PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord,
        },
    },
};
use multiemu_rom::{
    id::RomId,
    manager::{RomManager, RomRequirement},
};
use rand::RngCore;
use rangemap::RangeInclusiveMap;
use std::{
    borrow::Cow,
    io::Read,
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

const PAGE_SIZE: usize = 4096 - size_of::<RwLock<()>>();

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
    // The maximum word size
    pub max_word_size: usize,
    // Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<usize>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceHandle,
    // Initial contents
    pub initial_contents: RangeInclusiveMap<usize, StandardMemoryInitialContents>,
}

#[derive(Debug)]
pub struct StandardMemory {
    memory_operation_callbacks: Arc<StandardMemoryCallbacks>,
    rom_manager: Arc<RomManager>,
    environment: Arc<RwLock<Environment>>,
    pub memory_handle: MemoryHandle,
}

impl Component for StandardMemory {
    fn reset(&self) {
        self.memory_operation_callbacks
            .initialize_buffer(&self.rom_manager, &self.environment.read().unwrap());
    }
}

impl<R: RenderApi> ComponentConfig<R> for StandardMemoryConfig {
    type Component = StandardMemory;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&self.max_word_size),
            "Invalid word size"
        );
        assert!(
            !self.assigned_range.is_empty(),
            "Memory assigned must be non-empty"
        );

        let essentials = component_builder.essentials();

        let buffer_size = self.assigned_range.clone().count();
        let chunks_needed = buffer_size.div_ceil(PAGE_SIZE);
        let buffer =
            Vec::from_iter(std::iter::repeat_n([0; PAGE_SIZE], chunks_needed).map(RwLock::new));
        let assigned_range = self.assigned_range.clone();
        let assigned_address_space = self.assigned_address_space;

        let memory_operation_callbacks = Arc::new(StandardMemoryCallbacks {
            config: self.clone(),
            buffer: buffer.into_iter().collect(),
        });
        memory_operation_callbacks.initialize_buffer(
            &essentials.rom_manager,
            &essentials.environment.read().unwrap(),
        );

        let (component_builder, memory_handle) = match (self.readable, self.writable) {
            (true, true) => component_builder.insert_memory(
                memory_operation_callbacks.clone(),
                [(assigned_address_space, assigned_range)],
            ),
            (true, false) => component_builder.insert_read_memory(
                memory_operation_callbacks.clone(),
                [(assigned_address_space, assigned_range)],
            ),
            (false, true) => component_builder.insert_write_memory(
                memory_operation_callbacks.clone(),
                [(assigned_address_space, assigned_range)],
            ),
            (false, false) => {
                panic!("What?");
            }
        };

        component_builder.build_global(StandardMemory {
            memory_operation_callbacks,
            memory_handle,
            rom_manager: essentials.rom_manager.clone(),
            environment: essentials.environment.clone(),
        });
    }
}

#[derive(Debug)]
struct StandardMemoryCallbacks {
    config: StandardMemoryConfig,
    buffer: Vec<RwLock<[u8; PAGE_SIZE]>>,
}

impl ReadMemory for StandardMemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), RangeInclusiveMap<usize, ReadMemoryRecord>> {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_before_range,
                    ReadMemoryRecord::Denied,
                )]));
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_after_range,
                    ReadMemoryRecord::Denied,
                )]));
            }
        }

        self.read_internal(address, buffer);

        Ok(())
    }

    fn preview_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), RangeInclusiveMap<usize, PreviewMemoryRecord>> {
        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_before_range,
                    PreviewMemoryRecord::Denied,
                )]));
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_after_range,
                    PreviewMemoryRecord::Denied,
                )]));
            }
        }

        self.read_internal(address, buffer);

        Ok(())
    }
}

impl WriteMemory for StandardMemoryCallbacks {
    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), RangeInclusiveMap<usize, WriteMemoryRecord>> {
        debug_assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        if let Some(end_address) = self.config.assigned_range.start().checked_sub(1) {
            let invalid_before_range = address..=end_address;

            if !invalid_before_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_before_range,
                    WriteMemoryRecord::Denied,
                )]));
            }
        }

        if let Some(start_address) = self.config.assigned_range.end().checked_add(1) {
            let invalid_after_range = start_address..=address;

            if !invalid_after_range.is_empty() {
                return Err(RangeInclusiveMap::from_iter([(
                    invalid_after_range,
                    WriteMemoryRecord::Denied,
                )]));
            }
        }

        // Shoved off in a helper function to prevent duplicated logic
        self.write_internal(address, buffer);

        Ok(())
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

    fn initialize_buffer(&self, rom_manager: &RomManager, environment: &Environment) {
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
                    let mut rom_file = rom_manager
                        .open(
                            *rom_id,
                            RomRequirement::Required,
                            &environment.roms_directory,
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
        multiemu_machine::utils::set_main_thread();

        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::new(environment.clone());

        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager.clone(),
            environment.clone(),
            shader_cache.clone(),
        )
        .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=3,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=3,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
            },
        );
        let machine = machine.build(Default::default());

        let mut buffer = [0; 4];

        machine
            .memory_translation_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);

        let (machine, cpu_address_space) = MachineBuilder::<SoftwareRendering>::new(
            GameSystem::Unknown,
            rom_manager.clone(),
            environment.clone(),
            shader_cache.clone(),
        )
        .insert_address_space("cpu", 64);

        let (machine, _) = machine.insert_component(
            "workram",
            StandardMemoryConfig {
                max_word_size: 8,
                readable: true,
                writable: true,
                assigned_range: 0..=3,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=3,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
            },
        );
        let machine = machine.build(Default::default());

        let mut buffer = [0; 4];

        machine
            .memory_translation_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 4]);
    }

    #[test]
    fn basic_read() {
        multiemu_machine::utils::set_main_thread();

        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::new(environment.clone());

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
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=7,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
            },
        );
        let machine = machine.build(Default::default());

        let mut buffer = [0; 8];

        machine
            .memory_translation_table
            .read(0, cpu_address_space, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
    }

    #[test]
    fn basic_write() {
        multiemu_machine::utils::set_main_thread();

        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::new(environment.clone());

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
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=7,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
            },
        );
        let machine = machine.build(Default::default());

        let buffer = [0; 8];

        machine
            .memory_translation_table
            .write(0, cpu_address_space, &buffer)
            .unwrap();
    }

    #[test]
    fn basic_read_write() {
        multiemu_machine::utils::set_main_thread();

        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::new(environment.clone());

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
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=7,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
            },
        );
        let machine = machine.build(Default::default());

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
        multiemu_machine::utils::set_main_thread();

        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = ShaderCache::new(environment.clone());

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
                assigned_range: 0..=0xffff,
                assigned_address_space: cpu_address_space,
                initial_contents: RangeInclusiveMap::from_iter([(
                    0..=0xffff,
                    StandardMemoryInitialContents::Value(0xff),
                )]),
            },
        );
        let machine = machine.build(Default::default());

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
