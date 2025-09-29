use multiemu_rom::{RomId, RomRequirement};
use multiemu_runtime::{
    builder::ComponentBuilder,
    component::{BuildError, Component, ComponentConfig},
    memory::{Address, AddressSpaceId, MemoryOperationError, ReadMemoryRecord},
    platform::Platform,
};
use std::{
    fmt::Debug,
    fs::File,
    ops::RangeInclusive,
    sync::{Arc, LazyLock},
};

#[allow(unused)]
mod file;
#[cfg(feature = "mmap")]
mod mmap;

#[cfg(feature = "mmap")]
pub type DefaultRomMemoryBackend = mmap::MmapBackend;
#[cfg(not(feature = "mmap"))]
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
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        let adjusted_offset =
            (address - self.config.assigned_range.start()) + self.config.rom_range.start();

        debug_assert!(self.config.rom_range.contains(&adjusted_offset));

        self.backend.read(adjusted_offset, buffer);

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

        let rom = rom_manager
            .open(self.rom, RomRequirement::Required)
            .unwrap();

        let assigned_address_space = self.assigned_address_space;
        let assigned_range = self.assigned_range.clone();

        let component_builder =
            component_builder.memory_map_read(assigned_address_space, assigned_range);

        let backend = ROM_CACHE
            .0
            .entry_sync(self.rom)
            .or_put_with(|| {
                let backend = DefaultRomMemoryBackend::new(rom);
                Arc::new(backend)
            })
            .1
            .clone();

        component_builder.build(RomMemory {
            config: self,
            backend,
        });

        Ok(())
    }
}

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
