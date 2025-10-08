use crate::{
    graphics::GraphicsApi,
    machine::{builder::ComponentBuilder, registry::ComponentRegistry},
    memory::{
        Address, AddressSpaceId, PreviewMemoryError, PreviewMemoryErrorType, ReadMemoryError,
        ReadMemoryErrorType, WriteMemoryError, WriteMemoryErrorType,
    },
    platform::Platform,
};
use nalgebra::SVector;
use nohash::IsEnabled;
use ringbuffer::AllocRingBuffer;
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    borrow::Cow,
    fmt::Debug,
    hash::Hash,
    io::{Read, Write},
    num::NonZero,
    sync::Arc,
};

pub use path::{ComponentPath, ResourcePath};

mod path;

#[allow(unused)]
/// Basic supertrait for all components
pub trait Component: Send + Sync + Debug + Any {
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
    ) -> Result<(), ReadMemoryError> {
        Err(ReadMemoryError(
            std::iter::once((
                address..=(address + (buffer.len() - 1)),
                ReadMemoryErrorType::Denied,
            ))
            .collect(),
        ))
    }

    /// Previews memory at the specified address in the specified address space to fill the buffer
    fn preview_memory(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), PreviewMemoryError> {
        // Convert between a read and a preview

        self.read_memory(address, address_space, buffer)
            .map_err(|e| {
                PreviewMemoryError(
                    e.0.into_iter()
                        .map(|(range, record)| {
                            (
                                range,
                                match record {
                                    ReadMemoryErrorType::Denied => PreviewMemoryErrorType::Denied,
                                    ReadMemoryErrorType::OutOfBus => {
                                        PreviewMemoryErrorType::OutOfBus
                                    }
                                },
                            )
                        })
                        .collect(),
                )
            })
    }

    fn write_memory(
        &mut self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        Err(WriteMemoryError(
            std::iter::once((
                address..=(address + (buffer.len() - 1)),
                WriteMemoryErrorType::Denied,
            ))
            .collect(),
        ))
    }

    /// TODO: If we can figure out a non nightmarish way to have the platform generic references with these I
    /// would be happy for this

    /// The [Any] needs to be cast to a &[GraphicsApi::FramebufferTexture]
    fn access_framebuffer<'a>(
        &'a mut self,
        display_path: &ResourcePath,
        callback: Box<dyn FnOnce(&dyn Any) + 'a>,
    ) {
    }

    fn drain_samples(
        &mut self,
        audio_output_path: &ResourcePath,
    ) -> Option<&mut AllocRingBuffer<SVector<f32, 2>>> {
        None
    }
}

#[allow(unused)]
/// Factory config to construct a component
pub trait ComponentConfig<P: Platform>: Debug + Sized {
    /// Paramters to create this component
    type Component: Component;

    /// Make a new component from the config
    #[must_use]
    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, BuildError>;
}

#[derive(Debug)]
pub struct LateInitializedData<P: Platform> {
    pub component_graphics_initialization_data: <P::GraphicsApi as GraphicsApi>::InitializationData,
    pub component_registry: Arc<ComponentRegistry>,
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
