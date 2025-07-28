use crate::{
    builder::ComponentBuilder,
    memory::{
        Address, AddressSpaceHandle, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
        WriteMemoryRecord,
    },
    platform::Platform,
};
use multiemu_graphics::GraphicsApi;
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::{any::Any, borrow::Cow, fmt::Debug, hash::Hash, num::NonZero};

pub use component_ref::ComponentRef;
pub use path::*;
pub use registry::*;

mod component_ref;
mod path;
mod registry;

#[allow(unused)]
/// Basic supertrait for all components
pub trait Component: Debug + Any {
    /// Called when machine initialization is finished
    ///
    /// This is where you should do graphics initialization or anything that reads or writes from the memory translation table
    fn runtime_ready(&self) {}

    /// Reset state
    fn reset(&self) {}

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

    /// Make a new component from the config
    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<(), BuildError>;
}

#[derive(Debug)]
pub struct LateInitializedData<P: Platform> {
    pub component_graphics_initialization_data: <P::GraphicsApi as GraphicsApi>::InitializationData,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct ComponentId(NonZero<u16>);

impl Hash for ComponentId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.0.get());
    }
}

impl IsEnabled for ComponentId {}

#[derive(thiserror::Error, Debug)]
pub enum SaveError {
    #[error("Invalid version")]
    InvalidVersion,
    #[error("Invalid data")]
    InvalidData,
}

#[derive(thiserror::Error, Debug)]
pub enum SnapshotError {
    #[error("Invalid version")]
    InvalidVersion,
    #[error("Invalid data")]
    InvalidData,
}

#[derive(thiserror::Error, Debug)]
pub enum BuildError {
    #[error("Save error: {0:#?}")]
    LoadingSave(#[from] SaveError),
    #[error("Invalid config {0:}")]
    InvalidConfig(Cow<'static, str>),
    #[error("IO error: {0:#?}")]
    IoError(#[from] std::io::Error),
}

/// Version that components use
pub type ComponentVersion = u64;
