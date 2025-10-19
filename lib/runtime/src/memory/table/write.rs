use super::{MemoryAccessTable, address_space::AddressSpaceId};
use crate::memory::Address;
use multiemu_range::ContiguousRange;
use multiemu_range::RangeIntersection;
use num::traits::ToBytes;
use rangemap::RangeInclusiveMap;
use std::ops::RangeInclusive;
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

impl MemoryAccessTable {
    /// Step through the memory translation table to give a set of components the buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn write(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        let buffer_subrange = RangeInclusive::from_start_and_length(0, buffer.len());
        if buffer.is_empty() {
            return Ok(());
        }

        let address_space_info = self
            .address_spaces
            .get(address_space.0 as usize)
            .ok_or_else({
                let buffer_subrange = buffer_subrange.clone();

                move || {
                    WriteMemoryError(RangeInclusiveMap::from_iter([(
                        (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                        WriteMemoryErrorType::OutOfBus,
                    )]))
                }
            })?;

        let width_mask = address_space_info.width_mask;
        let address_masked = address & width_mask;
        let end_address = address_masked + buffer.len() - 1;

        // Check for wraparound
        if end_address > width_mask {
            let first_len = width_mask - address_masked + 1;
            let (first_part, second_part) = buffer.split_at(first_len);

            self.write(address_masked, address_space, first_part)?;
            self.write(0, address_space, second_part)?;

            return Ok(());
        }

        let access_range =
            (buffer_subrange.start() + address_masked)..=(buffer_subrange.end() + address_masked);
        let members = address_space_info.get_members();

        members.write.visit_overlapping(
            access_range.clone(),
            #[inline]
            |entry_assigned_range, mirror_start, component| {
                let component_access_range = entry_assigned_range.intersection(&access_range);
                let offset = (*component_access_range.start() - *entry_assigned_range.start())
                    ..=(*component_access_range.end() - *entry_assigned_range.start());

                // Determine base: mirror offset or source
                let operation_base = mirror_start.unwrap_or(*entry_assigned_range.start());

                // Adjust buffer slice
                let buffer_range = (*component_access_range.start() - access_range.start())
                    ..=(*component_access_range.end() - access_range.start());
                let adjusted_buffer = &buffer[buffer_range];

                component.write().write_memory(
                    operation_base + offset.start(),
                    address_space,
                    adjusted_buffer,
                )
            },
        )?;

        Ok(())
    }

    #[inline]
    /// Helper function to write with a little endian value
    pub fn write_le_value<T: ToBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        value: T,
    ) -> Result<(), WriteMemoryError> {
        self.write(address, address_space, value.to_le_bytes().as_ref())
    }

    #[inline]
    /// Helper function to write with a big endian value
    pub fn write_be_value<T: ToBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        value: T,
    ) -> Result<(), WriteMemoryError> {
        self.write(address, address_space, value.to_be_bytes().as_ref())
    }
}
