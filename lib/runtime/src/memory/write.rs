use std::ops::RangeInclusive;

use super::AddressSpace;
use crate::memory::Address;
use multiemu_range::{ContiguousRange, RangeIntersection};
use num::traits::ToBytes;
use rangemap::RangeInclusiveMap;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Why a write operation failed
pub enum WriteMemoryErrorType {
    /// Access was denied
    Denied,
    /// Nothing is mapped there
    OutOfBus,
}

#[derive(Error, Debug)]
#[error("Write operation failed: {0:#x?}")]
/// Wrapper around the error type in order to specific ranges
pub struct WriteMemoryError(pub RangeInclusiveMap<Address, WriteMemoryErrorType>);

impl AddressSpace {
    /// Step through the memory translation table to give a set of components the buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn write(&self, mut address: Address, buffer: &[u8]) -> Result<(), WriteMemoryError> {
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
                    members.write.visit_overlapping(
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
                            let adjusted_buffer = &remaining_buffer[buffer_range];

                            component.interact_mut(|component| {
                                component.write_memory(
                                    operation_base + offset,
                                    self.id,
                                    adjusted_buffer,
                                )
                            })
                        },
                    )
                },
            )?;

            // Move forward in the buffer
            remaining_buffer = &remaining_buffer[chunk_len..];
            address = (address_masked + chunk_len) & self.width_mask;
        }

        Ok(())
    }

    #[inline]
    /// Helper function to write with a little endian value
    pub fn write_le_value<T: ToBytes>(
        &self,
        address: Address,
        value: T,
    ) -> Result<(), WriteMemoryError> {
        self.write(address, value.to_le_bytes().as_ref())
    }

    #[inline]
    /// Helper function to write with a big endian value
    pub fn write_be_value<T: ToBytes>(
        &self,
        address: Address,
        value: T,
    ) -> Result<(), WriteMemoryError> {
        self.write(address, value.to_be_bytes().as_ref())
    }
}
