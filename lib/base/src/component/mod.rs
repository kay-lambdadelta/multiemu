use crate::{
    graphics::GraphicsApi,
    machine::builder::ComponentBuilder,
    memory::{
        Address, AddressSpaceId, PreviewMemoryError, PreviewMemoryErrorType, ReadMemoryError,
        ReadMemoryErrorType, WriteMemoryError, WriteMemoryErrorType,
    },
    platform::Platform,
};
pub use handle::*;
use nalgebra::SVector;
pub use path::{ComponentPath, ResourcePath};
use ringbuffer::AllocRingBuffer;
use std::{
    any::Any,
    error::Error,
    fmt::Debug,
    io::{Read, Write},
};

mod handle;
mod path;

#[allow(unused)]
/// Basic supertrait for all components
pub trait Component: Send + Sync + Debug + Any {
    /// Tell the runtime what save version is current for this component
    fn save_version(&self) -> Option<ComponentVersion> {
        None
    }

    /// Tell the runtime what snapshot version is current for this component
    fn snapshot_version(&self) -> Option<ComponentVersion> {
        None
    }

    /// Write the save
    fn store_save(&self, writer: Box<dyn Write>) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Read the snapshot
    fn load_snapshot(
        &mut self,
        version: ComponentVersion,
        reader: Box<dyn Read>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    /// Store the snapshot
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

    /// Writes memory at the specified address in the specified address space
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

    // TODO: Find out a non nightmarish way to have the platform generic here

    #[allow(clippy::type_complexity)]
    /// The [Any] needs to be cast to a &[GraphicsApi::FramebufferTexture]
    fn access_framebuffer<'a>(
        &'a mut self,
        display_path: &ResourcePath,
        callback: Box<dyn FnOnce(&dyn Any) + 'a>,
    ) {
    }

    /// Give the runtime the audio sample ring buffer
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
    /// The component that this config will create
    type Component: Component;

    /// Make a new component from the config
    fn build_component(
        self,
        component_builder: ComponentBuilder<P, Self::Component>,
    ) -> Result<Self::Component, Box<dyn Error>>;
}

/// Data that the runtime will provide at the end of the initialization sequence
#[derive(Debug)]
pub struct LateInitializedData<P: Platform> {
    /// Graphics related data
    pub component_graphics_initialization_data: <P::GraphicsApi as GraphicsApi>::InitializationData,
}

/// Version that components use
pub type ComponentVersion = u64;
