use crate::{
    builder::ComponentBuilder,
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, MemoryTranslationTable,
        PreviewMemoryRecord, ReadMemoryRecord, WriteMemoryRecord,
    },
    platform::Platform,
};
use multiemu_graphics::{GraphicsApi, GraphicsContextFeatures};
use multiemu_rom::RomManager;
use num::rational::Ratio;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::{any::Any, fmt::Debug, num::NonZero, sync::Arc};

pub use component_ref::ComponentRef;
pub use store::*;

mod component_ref;
mod store;

/// Stuff every component optionally needs
#[derive(Debug)]
pub struct RuntimeEssentials<P: Platform> {
    /// The configured ROM manager
    pub rom_manager: Arc<RomManager>,
    /// The memory translation table
    pub memory_translation_table: Arc<MemoryTranslationTable>,
    /// This is not guarenteed to be initialized until [Component::on_runtime_ready] is called
    ///
    /// Therefore do not expect it to be filled out until then
    pub component_graphics_initialization_data: <P::GraphicsApi as GraphicsApi>::InitializationData,
    /// Sample rate for the audio hardware
    pub sample_rate: Ratio<u32>,
}

#[allow(unused)]
/// Basic supertrait for all components
pub trait Component: Debug + Any {
    /// Called when machine initialization is finished
    ///
    /// This is where you should do graphics initialization or anything that reads or writes from the memory translation table
    fn on_runtime_ready(&self) {}

    /// Reset state
    fn on_reset(&self) {}

    // Memory map related functions

    /// Reads memory at the specified address in the specified address space to fill the buffer
    fn read_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        Err(MemoryOperationError {
            records: RangeInclusiveMap::from_iter([(
                address..=(address + (buffer.len() - 1)),
                ReadMemoryRecord::Denied,
            )]),
            remap_callback: None,
        })
    }

    /// Previews memory at the specified address in the specified address space to fill the buffer
    fn preview_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<PreviewMemoryRecord>> {
        // Convert between a read and a preview

        self.read_memory(address, address_space, buffer)
            .map_err(|e| MemoryOperationError {
                records: e
                    .records
                    .into_iter()
                    .map(|(range, record)| {
                        (
                            range,
                            match record {
                                ReadMemoryRecord::Denied => PreviewMemoryRecord::Denied,
                                ReadMemoryRecord::Redirect {
                                    address,
                                    address_space,
                                } => PreviewMemoryRecord::Redirect {
                                    address,
                                    address_space,
                                },
                            },
                        )
                    })
                    .collect(),
                remap_callback: e.remap_callback,
            })
    }

    fn write_memory(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        Err(MemoryOperationError {
            records: RangeInclusiveMap::from_iter([(
                address..=(address + (buffer.len() - 1)),
                WriteMemoryRecord::Denied,
            )]),
            remap_callback: None,
        })
    }
}

#[allow(unused)]
/// Factory config to construct a component
pub trait ComponentConfig<P: Platform>: Debug + Send + Sync + Sized + 'static {
    /// Paramters to create this component
    type Component: Component;

    /// Components this one depends on for building
    fn build_dependencies(&self) -> impl IntoIterator<Item = ComponentId> {
        std::iter::empty()
    }

    /// Graphics components this depends on
    fn graphics_requirements(&self) -> GraphicsContextFeatures<P::GraphicsApi> {
        GraphicsContextFeatures::default()
    }

    /// Make a new component from the config
    fn build_component(
        self,
        component_ref: ComponentRef<Self::Component>,
        component_builder: ComponentBuilder<P, Self::Component>,
    );
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ComponentId(NonZero<u16>);

impl ComponentId {
    pub fn get(&self) -> u16 {
        self.0.get() - 1
    }
}