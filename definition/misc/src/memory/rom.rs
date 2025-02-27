use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, FromConfig, RuntimeEssentials},
    memory::{
        AddressSpaceId, VALID_MEMORY_ACCESS_SIZES,
        callbacks::{PreviewMemory, ReadMemory},
        memory_translation_table::{PreviewMemoryRecord, ReadMemoryRecord},
    },
};
use multiemu_rom::{id::RomId, manager::RomRequirement};
use rangemap::RangeInclusiveMap;
use std::{ops::RangeInclusive, sync::Arc};

#[derive(Debug)]
pub struct RomMemoryConfig {
    pub rom: RomId,
    /// The maximum word size
    pub max_word_size: u8,
    /// Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<usize>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceId,
}

#[derive(Debug)]
pub struct RomMemory;

impl Component for RomMemory {
    fn reset(&self) {
        // This is basically a stateless component so there isn't any need to reset
    }
}

impl FromConfig for RomMemory {
    type Config = RomMemoryConfig;
    type Quirks = ();

    fn from_config(
        component_builder: ComponentBuilder<Self>,
        essentials: Arc<RuntimeEssentials>,
        config: Self::Config,
        _quirks: Self::Quirks,
    ) {
        let rom_file = essentials
            .rom_manager()
            .open(config.rom, RomRequirement::Required)
            .unwrap();

        let assigned_address_space = config.assigned_address_space;
        let assigned_range = config.assigned_range.clone();

        #[cfg(platform_desktop)]
        let rom = unsafe { memmap2::MmapOptions::new().map(&rom_file).unwrap() };
        #[cfg(not(platform_desktop))]
        let rom = File::open(rom_file).unwrap();

        let memory_operation_callbacks = Arc::new(MemoryCallbacks { config, rom });

        component_builder
            .insert_read_memory(
                assigned_address_space,
                [assigned_range.clone()],
                memory_operation_callbacks.clone(),
            )
            .insert_preview_memory(
                assigned_address_space,
                [assigned_range.clone()],
                memory_operation_callbacks.clone(),
            )
            .build_global(Self);
    }
}

#[derive(Debug)]
struct MemoryCallbacks {
    config: RomMemoryConfig,
    #[cfg(platform_desktop)]
    rom: memmap2::Mmap,
    // FIXME: Finish fallback for mmap-less platforms
    #[cfg(not(platform_desktop))]
    rom: File,
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

        let affected_range = address..=(address + buffer.len() - 1);

        if buffer.len() > self.config.max_word_size as usize {
            errors.insert(affected_range.clone(), ReadMemoryRecord::Denied);
        }

        let adjusted_offset = address - self.config.assigned_range.start();
        buffer.copy_from_slice(
            &self.rom[adjusted_offset..=(adjusted_offset + buffer.len()).min(self.rom.len()) - 1],
        );
    }
}

impl PreviewMemory for MemoryCallbacks {
    fn preview_memory(
        &self,
        address: usize,
        buffer: &mut [u8],
        _address_space: AddressSpaceId,
        _errors: &mut RangeInclusiveMap<usize, PreviewMemoryRecord>,
    ) {
        let adjusted_offset = address - self.config.assigned_range.start();

        buffer.copy_from_slice(
            &self.rom[adjusted_offset..=(adjusted_offset + buffer.len()).min(self.rom.len()) - 1],
        );
    }
}
