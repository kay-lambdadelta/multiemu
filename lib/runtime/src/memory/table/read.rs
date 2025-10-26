use super::{MemoryAccessTable, address_space::AddressSpaceId};
use crate::memory::Address;
use multiemu_range::ContiguousRange;
use multiemu_range::RangeIntersection;
use num::traits::FromBytes;
use rangemap::RangeInclusiveMap;
use std::ops::RangeInclusive;
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

impl MemoryAccessTable {
    /// Step through the memory translation table to fill a buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    ///
    #[inline]
    pub fn read(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        avoid_side_effects: bool,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
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
                    ReadMemoryError(RangeInclusiveMap::from_iter([(
                        (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                        ReadMemoryErrorType::OutOfBus,
                    )]))
                }
            })?;

        let width_mask = address_space_info.width_mask;
        let address_masked = address & width_mask;
        let end_address = address_masked + buffer.len() - 1;

        // Check for wraparound
        if end_address > width_mask {
            let first_len = width_mask - address_masked + 1;
            let (first_part, second_part) = buffer.split_at_mut(first_len);

            self.read(
                address_masked,
                address_space,
                avoid_side_effects,
                first_part,
            )?;
            self.read(0, address_space, avoid_side_effects, second_part)?;

            return Ok(());
        }

        let access_range =
            (buffer_subrange.start() + address_masked)..=(buffer_subrange.end() + address_masked);
        let members = address_space_info.get_members();

        members.read.visit_overlapping(
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
                let adjusted_buffer = &mut buffer[buffer_range];

                component.interact(|component| {
                    component.read_memory(
                        operation_base + offset.start(),
                        address_space,
                        avoid_side_effects,
                        adjusted_buffer,
                    )
                })
            },
        )?;

        Ok(())
    }

    #[inline]
    pub fn read_le_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        avoid_side_effects: bool,
    ) -> Result<T, ReadMemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.read(address, address_space, avoid_side_effects, buffer.as_mut())?;
        Ok(T::from_le_bytes(&buffer))
    }

    #[inline]
    pub fn read_be_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        avoid_side_effects: bool,
    ) -> Result<T, ReadMemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.read(address, address_space, avoid_side_effects, buffer.as_mut())?;
        Ok(T::from_be_bytes(&buffer))
    }
}
