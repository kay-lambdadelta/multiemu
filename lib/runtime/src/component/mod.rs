use crate::{
    audio::AudioOutputId, builder::ComponentBuilder, graphics::FramebufferStorage, memory::{
        Address, AddressSpaceId, MemoryOperationError, PreviewMemoryRecord, ReadMemoryRecord,
        WriteMemoryRecord,
    }, platform::Platform
};
use multiemu_audio::SampleFormat;
use multiemu_graphics::GraphicsApi;
use nalgebra::SVector;
use nohash::IsEnabled;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    borrow::Cow,
    fmt::Debug,
    hash::Hash,
    io::{Read, Write},
    num::NonZero,
};

pub use component_ref::ComponentRef;
pub use path::*;
pub use registry::*;

mod component_ref;
mod path;
mod registry;

#[allow(unused)]
/// Basic supertrait for all components
pub trait Component: Debug + Any {
    /// Reset state
    fn reset(&mut self) {}

    fn save_version(&self) -> Option<ComponentVersion> {
        None
    }

    fn snapshot_version(&self) -> Option<ComponentVersion> {
        None
    }

    fn store_save(&self, writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    fn store_snapshot(&self, writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Reads memory at the specified address in the specified address space to fill the buffer
    fn read_memory(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), MemoryOperationError<ReadMemoryRecord>> {
        Err(MemoryOperationError::from_iter([(
            address..=(address + (buffer.len() - 1)),
            ReadMemoryRecord::Denied,
        )]))
    }

    /// Previews memory at the specified address in the specified address space to fill the buffer
    fn preview_memory(
        &self,
        address: Address,
        address_space: AddressSpaceId,
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
            })
    }

    fn write_memory(
        &mut self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryOperationError<WriteMemoryRecord>> {
        Err(MemoryOperationError::from_iter([(
            address..=(address + (buffer.len() - 1)),
            WriteMemoryRecord::Denied,
        )]))
    }

    // TODO: Add a callback to alert when the component has been remapped as soon as the memory translation table has the infastructure
}

#[allow(unused)]
/// Factory config to construct a component
pub trait ComponentConfig<P: Platform>: Debug + Send + Sync + Sized {
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
    pub graphics_manager: FramebufferStorage<P::GraphicsApi>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
/// A reference to a component
///
/// The guts are [NonZero] for layout optimization
pub struct ComponentId(NonZero<u16>);

impl ComponentId {
    pub(crate) fn new(id: u16) -> Self {
        Self(id.checked_add(1).and_then(NonZero::new).unwrap())
    }

    pub(crate) fn get(&self) -> u16 {
        self.0.get() - 1
    }
}

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
