use multiemu_rom::{RomId, RomRequirement};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig},
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
    },
    platform::Platform,
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
    /// Initial bank
    pub initial_bank: usize,
}

#[derive(Debug)]
pub struct RomMemory {
    config: RomMemoryConfig,
    rom: Mutex<File>,
    bank: usize,
}

impl RomMemory {
    /// Swap the current bank, bank size is the assigned range size
    pub fn swap_banks(&mut self, bank: usize) {
        self.bank = bank;
    }
}

impl Component for RomMemory {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let bank_size = self.config.assigned_range.clone().count();

        let adjusted_offset =
            (address - self.config.assigned_range.start()) + (self.bank * bank_size);

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
        let bank_size = self.config.assigned_range.clone().count();

        let adjusted_offset =
            (address - self.config.assigned_range.start()) + (self.bank * bank_size);

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
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<(), BuildError> {
        if self.assigned_range.is_empty() {
            return Err(BuildError::InvalidConfig(
                "Memory assigned must be non-empty".into(),
            ));
        }

        let rom_manager = component_builder.rom_manager();

        let rom = Mutex::new(
            rom_manager
                .open(self.rom, RomRequirement::Required)
                .unwrap(),
        );

        let assigned_address_space = self.assigned_address_space;
        let assigned_range = self.assigned_range.clone();

        let component_builder =
            component_builder.map_memory_read([(assigned_address_space, assigned_range)]);

        component_builder.build(RomMemory {
            bank: self.initial_bank,
            config: self,
            rom,
        });

        Ok(())
    }
}
