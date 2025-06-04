use multiemu_rom::{id::RomId, manager::RomRequirement};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{Component, ComponentConfig},
    memory::{
        Address,
        callbacks::{Memory, ReadMemory},
        memory_translation_table::{
            MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
            address_space::AddressSpaceHandle,
        },
    },
};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    ops::RangeInclusive,
    sync::Mutex,
};

#[derive(Debug)]
pub struct RomMemoryConfig {
    pub rom: RomId,
    /// Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<Address>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceHandle,
}

#[derive(Debug)]
pub struct RomMemory;

impl Component for RomMemory {}

impl<B: ComponentBuilder<Component = RomMemory>> ComponentConfig<B> for RomMemoryConfig {
    type Component = RomMemory;

    fn build_component(self, component_builder: B) -> B::BuildOutput {
        let essentials = component_builder.essentials();

        let rom = Mutex::new(
            essentials
                .rom_manager
                .open(self.rom, RomRequirement::Required)
                .unwrap(),
        );

        let assigned_address_space = self.assigned_address_space;
        let assigned_range = self.assigned_range.clone();

        let memory_operation_callbacks = MemoryCallbacks { config: self, rom };

        let (component_builder, _) = component_builder.insert_read_memory(
            memory_operation_callbacks,
            [(assigned_address_space, assigned_range)],
        );

        component_builder.build_global(RomMemory)
    }
}

#[derive(Debug)]
struct MemoryCallbacks {
    config: RomMemoryConfig,
    rom: Mutex<File>,
}

impl Memory for MemoryCallbacks {}

impl ReadMemory for MemoryCallbacks {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let adjusted_offset = address - self.config.assigned_range.start();

        let mut rom_guard = self.rom.lock().unwrap();
        rom_guard
            .seek(SeekFrom::Start(adjusted_offset as u64))
            .unwrap();
        rom_guard.read_exact(buffer).unwrap();

        Ok(())
    }

    fn preview_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        let adjusted_offset = address - self.config.assigned_range.start();

        let mut rom_guard = self.rom.lock().unwrap();
        rom_guard
            .seek(SeekFrom::Start(adjusted_offset as u64))
            .unwrap();
        rom_guard.read_exact(buffer).unwrap();

        Ok(())
    }
}
