use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{
        AddressSpaceId, VALID_MEMORY_ACCESS_SIZES,
        callbacks::{ReadMemory, WriteMemory},
        memory_translation_table::{ReadMemoryRecord, WriteMemoryRecord},
    },
};
use rangemap::RangeMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct MirrorMemoryConfig {
    pub readable: bool,
    pub writable: bool,
    pub assigned_ranges: RangeMap<usize, usize>,
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
        mut component_builder: ComponentBuilder<Self>,
        _essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let assigned_address_space = config.assigned_address_space;
        let assigned_ranges = config.assigned_ranges.clone();

        let readable = config.readable;
        let writable = config.writable;
        let memory_operation_callbacks = Arc::new(MemoryCallbacks { config });

        if readable {
            component_builder = component_builder.insert_read_memory(
                assigned_address_space,
                assigned_ranges.iter().map(|(range, _)| range.clone()),
                memory_operation_callbacks.clone(),
            );
        }

        if writable {
            component_builder = component_builder.insert_write_memory(
                assigned_address_space,
                assigned_ranges.iter().map(|(range, _)| range.clone()),
                memory_operation_callbacks,
            );
        }

        component_builder.build_global(Self);
    }
}

struct MemoryCallbacks {
    config: MirrorMemoryConfig,
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

        let affected_range = address..address + buffer.len();

        let redirect_base_address = self
            .config
            .assigned_ranges
            .get(&affected_range.start)
            .unwrap();
        let adjusted_redirect_base_address =
            redirect_base_address + (address - affected_range.start);

        errors.insert(
            affected_range,
            ReadMemoryRecord::Redirect {
                address: adjusted_redirect_base_address,
            },
        );
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
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        let affected_range = address..address + buffer.len();

        let redirect_base_address = self
            .config
            .assigned_ranges
            .get(&affected_range.start)
            .unwrap();
        let adjusted_redirect_base_address =
            redirect_base_address + (address - affected_range.start);

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
    use multiemu_machine::display::software::SoftwareRendering;
    use multiemu_machine::memory::AddressSpaceId;
    use multiemu_rom::manager::RomManager;
    use multiemu_rom::system::GameSystem;
    use rangemap::RangeMap;
    use std::sync::{Arc, RwLock};

    const ADDRESS_SPACE: AddressSpaceId = AddressSpaceId::new(0);

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
                    assigned_range: 0..0x10000,
                    assigned_address_space: ADDRESS_SPACE,
                    initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
                },
            )
            .insert_component::<MirrorMemory>(
                "workram-mirror",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    assigned_ranges: RangeMap::from_iter([(0x10000..0x20000, 0x0000)]),
                    assigned_address_space: ADDRESS_SPACE,
                },
            )
            .build::<SoftwareRendering>(Default::default());
        let mut buffer = [0; 8];

        machine
            .memory_translation_table()
            .read(0x10000, ADDRESS_SPACE, &mut buffer)
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
                    assigned_range: 0..0x10000,
                    assigned_address_space: ADDRESS_SPACE,
                    initial_contents: vec![StandardMemoryInitialContents::Value { value: 0xff }],
                },
            )
            .insert_component::<MirrorMemory>(
                "workram-mirror",
                MirrorMemoryConfig {
                    readable: true,
                    writable: true,
                    assigned_ranges: RangeMap::from_iter([(0x10000..0x20000, 0x0000)]),
                    assigned_address_space: ADDRESS_SPACE,
                },
            )
            .build::<SoftwareRendering>(Default::default());
        let buffer = [0; 8];

        machine
            .memory_translation_table()
            .write(0x10000, ADDRESS_SPACE, &buffer)
            .unwrap();
    }
}
