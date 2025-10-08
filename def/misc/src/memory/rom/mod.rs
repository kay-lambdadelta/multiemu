use multiemu::{
    component::{Component, ComponentConfig},
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, ReadMemoryError},
    platform::Platform,
    rom::{RomId, RomRequirement},
};
use std::{
    fmt::Debug,
    fs::File,
    ops::RangeInclusive,
    sync::{Arc, LazyLock},
};

#[allow(unused)]
mod file;
#[cfg(any(target_family = "unix", target_os = "windows"))]
mod mmap;

#[cfg(any(target_family = "unix", target_os = "windows"))]
pub type DefaultRomMemoryBackend = mmap::MmapBackend;
#[cfg(not(any(target_family = "unix", target_os = "windows")))]
pub type DefaultRomMemoryBackend = file::FileBackend;

/// Use a global cache to share among rom instances and reduce loaded memory/fds
static ROM_CACHE: LazyLock<RomCache<DefaultRomMemoryBackend>> = LazyLock::new(RomCache::default);

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
    backend: Arc<DefaultRomMemoryBackend>,
}

impl Component for RomMemory {
    fn read_memory(
        &self,
        address: Address,
        _address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        let adjusted_offset =
            (address - self.config.assigned_range.start()) + self.config.rom_range.start();

        self.backend.read(adjusted_offset, buffer);

        Ok(())
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

        let rom_manager = component_builder.rom_manager();

        let rom = rom_manager
            .open(self.rom, RomRequirement::Required)
            .unwrap();

        let assigned_address_space = self.assigned_address_space;
        let assigned_range = self.assigned_range.clone();

        let backend = ROM_CACHE
            .0
            .entry_sync(self.rom)
            .or_put_with(|| {
                let backend = DefaultRomMemoryBackend::new(rom);
                Arc::new(backend)
            })
            .1
            .clone();

        component_builder.memory_map_read(assigned_address_space, assigned_range);

        Ok(RomMemory {
            config: self,
            backend,
        })
    }
}

#[allow(unused)]
pub trait RomMemoryBackend: Debug + Send + Sync + Sized + 'static {
    fn new(file: File) -> Self;
    fn read(&self, address: usize, buffer: &mut [u8]);
}

pub struct RomCache<B: RomMemoryBackend>(pub scc::HashCache<RomId, Arc<B>>);

impl<B: RomMemoryBackend> Default for RomCache<B> {
    fn default() -> Self {
        Self(scc::HashCache::with_capacity(0, 4))
    }
}
