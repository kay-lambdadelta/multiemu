use std::ops::RangeInclusive;

use super::AddressSpace;
use crate::memory::Address;
use multiemu_range::{ContiguousRange, RangeIntersection};
use num::traits::FromBytes;
use rangemap::RangeInclusiveMap;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Why a read operation failed
pub enum ReadMemoryErrorType {
    /// Access was denied
    Denied,
    /// Nothing is mapped there
    OutOfBus,
    /// It would be impossible to view this memory without a state change
    Impossible,
}

#[derive(Error, Debug)]
#[error("Read operation failed: {0:#x?}")]
/// Wrapper around the error type in order to specify ranges
pub struct ReadMemoryError(pub RangeInclusiveMap<Address, ReadMemoryErrorType>);

impl AddressSpace {
    /// Step through the memory translation table to fill a buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    ///
    #[inline]
    pub fn read(
        &self,
        mut address: Address,
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        let mut remaining_buffer = buffer;

        while !remaining_buffer.is_empty() {
            let address_masked = address & self.width_mask;
            let end_address = address_masked + remaining_buffer.len() - 1;

            let chunk_len = if end_address > self.width_mask {
                // Wraparound
                self.width_mask - address_masked + 1
            } else {
                remaining_buffer.len()
            };

            let access_range = RangeInclusive::from_start_and_length(address_masked, chunk_len);

            self.interact_members(
                #[inline]
                |members| {
                    members.read.visit_overlapping(
                        access_range.clone(),
                        #[inline]
                        |entry_assigned_range, mirror_start, component| {
                            let component_access_range =
                                entry_assigned_range.intersection(&access_range);
                            let offset =
                                component_access_range.start() - entry_assigned_range.start();

                            let operation_base =
                                mirror_start.unwrap_or(*entry_assigned_range.start());

                            let buffer_range = (component_access_range.start()
                                - access_range.start())
                                ..=(component_access_range.end() - access_range.start());
                            let adjusted_buffer = &mut remaining_buffer[buffer_range];

                            component.interact(|component| {
                                component.read_memory(
                                    operation_base + offset,
                                    self.id,
                                    avoid_side_effects,
                                    adjusted_buffer,
                                )
                            })
                        },
                    )
                },
            )?;

            // Move forward in the buffer
            remaining_buffer = &mut remaining_buffer[chunk_len..];
            address = (address_masked + chunk_len) & self.width_mask;
        }

        Ok(())
    }

    /// Given a location, read a little endian value
    #[inline]
    pub fn read_le_value<T: FromBytes>(
        &self,
        address: Address,
        avoid_side_effects: bool,
    ) -> Result<T, ReadMemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.read(address, avoid_side_effects, buffer.as_mut())?;
        Ok(T::from_le_bytes(&buffer))
    }

    /// Given a location, read a big endian value
    #[inline]
    pub fn read_be_value<T: FromBytes>(
        &self,
        address: Address,
        avoid_side_effects: bool,
    ) -> Result<T, ReadMemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.read(address, avoid_side_effects, buffer.as_mut())?;
        Ok(T::from_be_bytes(&buffer))
    }
}
