use std::{
    any::Any,
    error::Error,
    fmt::Debug,
    io::{Read, Write},
    iter::Fuse,
    ops::RangeInclusive,
    sync::Weak,
};

pub use handle::*;
use multiemu_audio::FrameIterator;
use multiemu_range::ContiguousRange;

use crate::{
    graphics::GraphicsApi,
    machine::{Machine, builder::ComponentBuilder},
    memory::{Address, AddressSpaceId, MemoryError, MemoryErrorType},
    path::MultiemuPath,
    platform::Platform,
    scheduler::{EventQueue, Period},
};

mod handle;

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

    /// Reads memory at the specified address in the specified address space to
    /// fill the buffer
    fn memory_read(
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
    fn memory_write(
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
    fn access_framebuffer(&mut self, path: &MultiemuPath) -> &dyn Any {
        unreachable!()
    }

    /// Give the runtime the audio sample ring buffer
    fn get_audio_channel(&mut self, audio_output_path: &MultiemuPath) -> SampleSource<'_> {
        unreachable!()
    }

    /// Synchronize until the time tracker indicates that no more time can be consumed
    fn synchronize(&mut self, context: SynchronizationContext) {
        unreachable!()
    }

    /// Given a delta between this components time and real time, is the component as much as it can be
    fn needs_work(&self, delta: Period) -> bool {
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

    /// Do setup for subsystems that cannot be initalized during
    /// [`Self::build_component`]
    fn late_initialize(component: &mut Self::Component, data: &LateInitializedData<P>) {}
}

/// Data that the runtime will provide at the end of the initialization sequence
#[derive(Debug)]
pub struct LateInitializedData<P: Platform> {
    /// Pointer to the running machine
    pub machine: Weak<Machine>,
    /// Graphics related data
    pub component_graphics_initialization_data: <P::GraphicsApi as GraphicsApi>::InitializationData,
}

/// Version that components use
pub type ComponentVersion = u64;

#[derive(Debug)]
pub struct SynchronizationContext<'a> {
    event_queue: &'a EventQueue,
    updated_timestamp: &'a mut Period,
    current_timestamp: Period,
    last_attempted_allocation: &'a mut Option<Period>,
}

impl<'a> SynchronizationContext<'a> {
    #[inline]
    pub fn allocate<'b>(
        &'b mut self,
        period: Period,
        limit: Option<u64>,
    ) -> Fuse<QuantaIterator<'b, 'a>> {
        *self.last_attempted_allocation = Some(period);

        QuantaIterator {
            period,
            limit,
            context: self,
        }
        .fuse()
    }
}

pub struct QuantaIterator<'b, 'a> {
    period: Period,
    limit: Option<u64>,
    context: &'b mut SynchronizationContext<'a>,
}

impl Iterator for QuantaIterator<'_, '_> {
    type Item = Period;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(limit) = &mut self.limit {
            if *limit == 0 {
                return None;
            } else {
                *limit -= 1;
            }
        }

        let next_timestamp = *self.context.updated_timestamp + self.period;

        if next_timestamp <= self.context.current_timestamp
            && self.context.event_queue.within_deadline(next_timestamp)
        {
            *self.context.updated_timestamp = next_timestamp;

            Some(next_timestamp)
        } else {
            None
        }
    }
}

pub struct SampleSource<'a> {
    pub source: Box<dyn FrameIterator<f32, 1> + 'a>,
    pub sample_rate: f32,
}
