use multiemu_machine::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    display::backend::RenderApi,
    memory::{
        AddressSpaceHandle,
        callbacks::ReadMemory,
        memory_translation_table::{PreviewMemoryRecord, ReadMemoryRecord},
    },
};
use multiemu_rom::{id::RomId, manager::RomRequirement};
use rangemap::RangeInclusiveMap;
use std::ops::RangeInclusive;

#[derive(Debug)]
pub struct RomMemoryConfig {
    pub rom: RomId,
    /// Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<usize>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceHandle,
}

#[derive(Debug)]
pub struct RomMemory;

impl Component for RomMemory {}

impl<R: RenderApi> ComponentConfig<R> for RomMemoryConfig {
    type Component = RomMemory;

    fn build_component(self, component_builder: ComponentBuilder<R, Self::Component>) {
        let essentials = component_builder.essentials();

        let rom_file = essentials
            .rom_manager
            .open(
                self.rom,
                RomRequirement::Required,
                &essentials.environment.read().unwrap().roms_directory,
            )
            .unwrap();

        let assigned_address_space = self.assigned_address_space;
        let assigned_range = self.assigned_range.clone();

        #[cfg(platform_desktop)]
        let rom = unsafe { memmap2::MmapOptions::new().map(&rom_file).unwrap() };
        #[cfg(not(platform_desktop))]
        let rom = File::open(rom_file).unwrap();

        let memory_operation_callbacks = MemoryCallbacks { config: self, rom };

        essentials.memory_translation_table.insert_read_memory(
            memory_operation_callbacks,
            [(assigned_address_space, assigned_range)],
        );

        component_builder.build_global(RomMemory);
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
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), RangeInclusiveMap<usize, ReadMemoryRecord>> {
        let adjusted_offset = address - self.config.assigned_range.start();
        buffer.copy_from_slice(
            &self.rom
                [adjusted_offset..=(adjusted_offset + (buffer.len() - 1)).min(self.rom.len()) - 1],
        );

        Ok(())
    }

    fn preview_memory(
        &self,
        address: usize,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), RangeInclusiveMap<usize, PreviewMemoryRecord>> {
        let adjusted_offset = address - self.config.assigned_range.start();

        buffer.copy_from_slice(
            &self.rom
                [adjusted_offset..=(adjusted_offset + (buffer.len() - 1)).min(self.rom.len()) - 1],
        );

        Ok(())
    }
}
