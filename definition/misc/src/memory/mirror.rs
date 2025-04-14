use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{
        AddressSpaceId, VALID_MEMORY_ACCESS_SIZES,
        callbacks::Memory,
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use rangemap::RangeInclusiveMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct MirrorMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    pub assigned_ranges: RangeInclusiveMap<usize, usize>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceId,
}

#[derive(Debug)]
pub struct MirrorMemory;

impl Component for MirrorMemory {}

impl FromConfig for MirrorMemory {
    type Config = MirrorMemoryConfig;
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        _essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let assigned_address_space = config.assigned_address_space;
        let assigned_ranges = config.assigned_ranges.clone();

        let component_builder = component_builder.insert_memory(
            assigned_ranges
                .iter()
                .map(|(range, _)| (range.clone(), assigned_address_space)),
            MemoryCallbacks { config },
        );

        component_builder.build_global(Self);
    }
}

struct MemoryCallbacks {
    config: MirrorMemoryConfig,
}

impl Memory for MemoryCallbacks {
    fn read_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
        errors: &mut RangeInclusiveMap<usize, ReadMemoryRecord>,
    ) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        if !self.config.readable {
            errors.insert(
                address..=(address + (buffer.len() - 1)),
                ReadMemoryRecord::Denied,
            );
            return;
        }

        let affected_range = address..=(address + (buffer.len() - 1));

        let redirect_base_address = self
            .config
            .assigned_ranges
            .get(affected_range.start())
            .unwrap();

        let adjusted_redirect_base_address =
            redirect_base_address + (address - affected_range.start());

        errors.insert(
            affected_range,
            ReadMemoryRecord::Redirect {
                address: adjusted_redirect_base_address,
            },
        );
    }

    fn write_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceId,
        buffer: &[u8],
        errors: &mut RangeInclusiveMap<usize, WriteMemoryRecord>,
    ) {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        if !self.config.writable {
            errors.insert(
                address..=(address + (buffer.len() - 1)),
                WriteMemoryRecord::Denied,
            );
            return;
        }

        let affected_range = address..=(address + buffer.len() - 1);

        let redirect_base_address = self
            .config
            .assigned_ranges
            .get(affected_range.start())
            .unwrap();
        let adjusted_redirect_base_address =
            redirect_base_address + (address - affected_range.start());

        errors.insert(
            affected_range,
            WriteMemoryRecord::Redirect {
                address: adjusted_redirect_base_address,
            },
        );
    }
}

#[cfg(test)]
mod test {
    use crate::memory::mirror::{MirrorMemory, MirrorMemoryConfig};
    use crate::memory::standard::{
        StandardMemory, StandardMemoryConfig, StandardMemoryInitialContents,
    };
    use multiemu_config::Environment;
    use multiemu_machine::builder::MachineBuilder;
    use multiemu_machine::display::backend::software::SoftwareRendering;
    use multiemu_machine::display::shader::ShaderCache;
    use multiemu_machine::memory::AddressSpaceId;
    use multiemu_rom::manager::RomManager;
    use multiemu_rom::system::GameSystem;
    use rangemap::RangeInclusiveMap;
    use std::sync::{Arc, RwLock};

    const ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

    #[test]
    fn basic_read() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = Arc::new(ShaderCache::default());

        let machine =
            MachineBuilder::new(GameSystem::Unknown, rom_manager, environment, shader_cache)
                .insert_address_space(ADDRESS_SPACE, 64)
                .insert_component::<StandardMemory>(
                    "workram",
                    StandardMemoryConfig {
                        max_word_size: 8,
                        readable: true,
                        writable: true,
                        assigned_range: 0..=0xffff,
                        assigned_address_space: ADDRESS_SPACE,
                        initial_contents: vec![StandardMemoryInitialContents::Value {
                            value: 0xff,
                        }],
                    },
                )
                .insert_component::<MirrorMemory>(
                    "workram-mirror",
                    MirrorMemoryConfig {
                        readable: true,
                        writable: true,
                        assigned_ranges: RangeInclusiveMap::from_iter([(
                            0x10000..=0x1ffff,
                            0x0000,
                        )]),
                        assigned_address_space: ADDRESS_SPACE,
                    },
                )
                .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0; 8];

        machine
            .memory_translation_table
            .read(0x10000, ADDRESS_SPACE, &mut buffer)
            .unwrap();
        assert_eq!(buffer, [0xff; 8]);
    }

    #[test]
    fn basic_write() {
        let environment = Arc::new(RwLock::new(Environment::default()));
        let rom_manager = Arc::new(RomManager::new(None, None).unwrap());
        let shader_cache = Arc::new(ShaderCache::default());

        let machine =
            MachineBuilder::new(GameSystem::Unknown, rom_manager, environment, shader_cache)
                .insert_address_space(ADDRESS_SPACE, 64)
                .insert_component::<StandardMemory>(
                    "workram",
                    StandardMemoryConfig {
                        max_word_size: 8,
                        readable: true,
                        writable: true,
                        assigned_range: 0..=0xffff,
                        assigned_address_space: ADDRESS_SPACE,
                        initial_contents: vec![StandardMemoryInitialContents::Value {
                            value: 0xff,
                        }],
                    },
                )
                .insert_component::<MirrorMemory>(
                    "workram-mirror",
                    MirrorMemoryConfig {
                        readable: true,
                        writable: true,
                        assigned_ranges: RangeInclusiveMap::from_iter([(
                            0x10000..=0x1ffff,
                            0x0000,
                        )]),
                        assigned_address_space: ADDRESS_SPACE,
                    },
                )
                .build::<SoftwareRendering>(Default::default());
        let buffer = [0; 8];

        machine
            .memory_translation_table
            .write(0x10000, ADDRESS_SPACE, &buffer)
            .unwrap();
    }
}
