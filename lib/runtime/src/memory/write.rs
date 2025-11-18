use std::ops::RangeInclusive;

use multiemu_range::{ContiguousRange, RangeIntersection};
use num::traits::ToBytes;

use super::AddressSpace;
use crate::memory::{
    Address, AddressSpaceCache, ComputedTablePageTarget, Members, MemoryError, MemoryErrorType,
    overlapping::Item,
};

impl AddressSpace {
    #[inline(always)]
    pub(super) fn write_internal(
        &self,
        mut address: Address,
        buffer: &[u8],
        members: &Members,
    ) -> Result<(), MemoryError> {
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
            let mut handled = false;

            for Item {
                entry_assigned_range,
                target,
            } in members.write.overlapping(access_range.clone())
            {
                handled = true;

                match target {
                    ComputedTablePageTarget::Component {
                        mirror_start,
                        component,
                    } => {
                        let component_access_range =
                            entry_assigned_range.intersection(&access_range);
                        let offset = component_access_range.start() - entry_assigned_range.start();

                        let operation_base = mirror_start.unwrap_or(*entry_assigned_range.start());

                        let buffer_range = (component_access_range.start() - access_range.start())
                            ..=(component_access_range.end() - access_range.start());
                        let adjusted_buffer = &remaining_buffer[buffer_range];

                        component.interact_mut(
                            #[inline]
                            |component| {
                                component.memory_write(
                                    operation_base + offset,
                                    self.id,
                                    adjusted_buffer,
                                )
                            },
                        )?;
                    }
                    ComputedTablePageTarget::Memory(_) => {
                        unreachable!()
                    }
                }
            }

            if !handled {
                return Err(MemoryError(
                    std::iter::once((access_range, MemoryErrorType::Denied)).collect(),
                ));
            }

            // Move forward in the buffer
            remaining_buffer = &remaining_buffer[chunk_len..];
            address = (address_masked + chunk_len) & self.width_mask;
        }

        Ok(())
    }

    /// Given a location, read a little endian value
    #[inline]
    pub(super) fn write_le_value_internal<T: ToBytes>(
        &self,
        address: Address,
        value: T,
        members: &Members,
    ) -> Result<(), MemoryError> {
        self.write_internal(address, value.to_le_bytes().as_ref(), members)
    }

    /// Given a location, read a big endian value
    #[inline]
    pub(super) fn write_be_value_internal<T: ToBytes>(
        &self,
        address: Address,
        value: T,
        members: &Members,
    ) -> Result<(), MemoryError> {
        self.write_internal(address, value.to_be_bytes().as_ref(), members)
    }

    /// Step through the memory translation table to fill a buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn write(
        &self,
        address: Address,
        cache: Option<&mut AddressSpaceCache>,
        buffer: &[u8],
    ) -> Result<(), MemoryError> {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.write_internal(address, buffer, members)
        } else {
            let members = self.members.load();
            self.write_internal(address, buffer, &members)
        }
    }

    /// Given a location, read a little endian value
    #[inline]
    pub fn write_le_value<T: ToBytes>(
        &self,
        address: Address,
        cache: Option<&mut AddressSpaceCache>,
        value: T,
    ) -> Result<(), MemoryError> {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.write_le_value_internal(address, value, members)
        } else {
            let members = self.members.load();
            self.write_le_value_internal(address, value, &members)
        }
    }

    /// Given a location, read a big endian value
    #[inline]
    pub fn write_be_value<T: ToBytes>(
        &self,
        address: Address,
        cache: Option<&mut AddressSpaceCache>,
        value: T,
    ) -> Result<(), MemoryError> {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.write_be_value_internal(address, value, members)
        } else {
            let members = self.members.load();
            self.write_be_value_internal(address, value, &members)
        }
    }
}
