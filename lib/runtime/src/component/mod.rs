use std::{
    any::Any,
    error::Error,
    fmt::Debug,
    io::{Read, Write},
    ops::RangeInclusive,
};

pub use handle::*;
use multiemu_range::ContiguousRange;
use nalgebra::SVector;
pub use path::{ComponentPath, ResourcePath};
use ringbuffer::AllocRingBuffer;

use crate::{
    graphics::GraphicsApi,
    machine::builder::ComponentBuilder,
    memory::{Address, AddressSpaceId, MemoryError, MemoryErrorType},
    platform::Platform,
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
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        Err(MemoryError(
            std::iter::once((
                RangeInclusive::from_start_and_length(address, buffer.len()),
                MemoryErrorType::Denied,
            ))
            .collect(),
        ))
    }

    /// Writes memory at the specified address in the specified address space
    fn write_memory(
        &mut self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), MemoryError> {
        Err(MemoryError(
            std::iter::once((
                RangeInclusive::from_start_and_length(address, buffer.len()),
                MemoryErrorType::Denied,
            ))
            .collect(),
        ))
    }

    // TODO: Find out a non nightmarish way to have the platform generic here

    /// The [Any] needs to be cast to a [`GraphicsApi::FramebufferTexture`]
    fn access_framebuffer(&mut self, path: &ResourcePath) -> &dyn Any {
        unreachable!()
    }

    /// Give the runtime the audio sample ring buffer
    fn drain_samples(
        &mut self,
        audio_output_path: &ResourcePath,
    ) -> &mut AllocRingBuffer<SVector<f32, 2>> {
        unreachable!()
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
