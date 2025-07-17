use multiemu_rom::{RomId, RomRequirement};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig, ComponentRef},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
    },
    platform::Platform,
};
use multiemu_save::ComponentSave;
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
pub struct RomMemory {
    config: RomMemoryConfig,
    rom: Mutex<File>,
}

impl Component for RomMemory {
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

impl<P: Platform> ComponentConfig<P> for RomMemoryConfig {
    type Component = RomMemory;

    fn build_component(
        self,
        _component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
        _save: Option<ComponentSave>,
    ) -> Result<(), BuildError> {
        if self.assigned_range.is_empty() {
            return Err(BuildError::InvalidConfig(
                "Memory assigned must be non-empty".into(),
            ));
        }

        let essentials = component_builder.essentials();

        let rom = Mutex::new(
            essentials
                .rom_manager
                .open(self.rom, RomRequirement::Required)
                .unwrap(),
        );

        let assigned_address_space = self.assigned_address_space;
        let assigned_range = self.assigned_range.clone();

        let component_builder =
            component_builder.map_memory_read([(assigned_address_space, assigned_range)]);

        component_builder.build_global(RomMemory { config: self, rom });

        Ok(())
    }
}
