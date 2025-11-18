use std::{fmt::Debug, fs::File, ops::RangeInclusive, sync::LazyLock};

use bytes::Bytes;
use multiemu_runtime::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, MemoryError},
    platform::Platform,
    program::{RomId, RomRequirement},
};

#[cfg(any(target_family = "unix", target_os = "windows"))]
mod mmap;
#[allow(unused)]
mod vec;

#[cfg(any(target_family = "unix", target_os = "windows"))]
pub type DefaultRomMemoryBackend = mmap::MmapBackend;
#[cfg(not(any(target_family = "unix", target_os = "windows")))]
pub type DefaultRomMemoryBackend = vec::VecBackend;

/// Use a global cache to share among rom instances and reduce loaded memory/fds
static ROM_CACHE: LazyLock<RomCache> = LazyLock::new(RomCache::default);

#[derive(Debug)]
pub struct RomMemoryConfig {
    pub rom: RomId,
    /// Memory region this buffer will be mapped to
    pub assigned_range: RangeInclusive<Address>,
    /// Address space this exists on
    pub assigned_address_space: AddressSpaceId,
    /// ROM range
    pub rom_range: RangeInclusive<usize>,
}

#[derive(Debug)]
pub struct RomMemory {
    config: RomMemoryConfig,
    bytes: Bytes,
}

impl RomMemory {
    pub fn set_rom_range(&mut self, range: RangeInclusive<Address>) {
        assert_eq!(
            self.config.rom_range.clone().count(),
            range.clone().count(),
            "New range does not represent the same space as the old range"
        );

        self.config.rom_range = range;
    }
}

impl Component for RomMemory {
    fn memory_read(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        _avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        todo!()
    }
}

impl<P: Platform> ComponentConfig<P> for RomMemoryConfig {
    type Component = RomMemory;

    fn build_component(
        self,
        component_builder: ComponentBuilder<'_, P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn std::error::Error>> {
        if self.assigned_range.is_empty() {
            return Err("Memory assigned must be non-empty".into());
        }

        let program_manager = component_builder.program_manager();

        let rom = program_manager
            .open(self.rom, RomRequirement::Required)
            .unwrap();

        let assigned_address_space = self.assigned_address_space;
        let assigned_range = self.assigned_range.clone();

        let bytes = ROM_CACHE
            .0
            .entry_sync(self.rom)
            .or_put_with(|| DefaultRomMemoryBackend::open(rom))
            .1
            .clone();
        let bytes = bytes.slice(self.rom_range.clone());

        let (component_builder, buffer_path) = component_builder.memory_register_buffer(
            assigned_address_space,
            "buffer",
            bytes.clone(),
        );
        component_builder.memory_map_buffer_read(
            assigned_address_space,
            assigned_range,
            &buffer_path,
        );

        Ok(RomMemory {
            config: self,
            bytes,
        })
    }
}

#[allow(unused)]
pub trait RomMemoryBackend: Debug + Send + Sync + Sized + 'static {
    fn open(file: File) -> Bytes;
}

pub struct RomCache(pub scc::HashCache<RomId, Bytes>);

impl Default for RomCache {
    fn default() -> Self {
        Self(scc::HashCache::with_capacity(0, 4))
    }
}
