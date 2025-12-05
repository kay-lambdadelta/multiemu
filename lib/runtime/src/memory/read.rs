use std::ops::RangeInclusive;

use multiemu_range::{ContiguousRange, RangeIntersection};
use num::traits::FromBytes;

use super::AddressSpace;
use crate::{
    memory::{
        Address, AddressSpaceCache, ComputedTablePageTarget, Members, MemoryError, MemoryErrorType,
        overlapping::Item,
    },
    scheduler::Period,
};

impl AddressSpace {
    /// Force code into the generic read_*_value functions
    #[inline(always)]
    pub(super) fn read_internal(
        &self,
        mut address: Address,
        current_timestamp: Period,
        members: &Members,
        avoid_side_effects: bool,
        buffer: &mut [u8],
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
            } in members.read.overlapping(access_range.clone())
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
                        let adjusted_buffer = &mut remaining_buffer[buffer_range];

                        component.interact(
                            current_timestamp,
                            #[inline]
                            |component| {
                                component.memory_read(
                                    operation_base + offset,
                                    self.id,
                                    avoid_side_effects,
                                    adjusted_buffer,
                                )
                            },
                        )?;
                    }
                    ComputedTablePageTarget::Memory(bytes) => {
                        let memory_access_range = entry_assigned_range.intersection(&access_range);

                        let memory_offset =
                            memory_access_range.start() - entry_assigned_range.start();
                        let buffer_range = (memory_access_range.start() - access_range.start())
                            ..=(memory_access_range.end() - access_range.start());

                        let adjusted_buffer = &mut remaining_buffer[buffer_range];
                        let memory_range = RangeInclusive::from_start_and_length(
                            memory_offset,
                            adjusted_buffer.len(),
                        );

                        adjusted_buffer.copy_from_slice(&bytes[memory_range]);
                    }
                }
            }

            if !handled {
                return Err(MemoryError(
                    std::iter::once((access_range, MemoryErrorType::Denied)).collect(),
                ));
            }

            // Move forward in the buffer
            remaining_buffer = &mut remaining_buffer[chunk_len..];
            address = (address_masked + chunk_len) & self.width_mask;
        }

        Ok(())
    }

    /// Given a location, read a little endian value
    #[inline(always)]
    pub(super) fn read_le_value_internal<T: FromBytes>(
        &self,
        address: Address,
        current_timestamp: Period,
        members: &Members,
        avoid_side_effects: bool,
    ) -> Result<T, MemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.read_internal(
            address,
            current_timestamp,
            members,
            avoid_side_effects,
            buffer.as_mut(),
        )?;
        Ok(T::from_le_bytes(&buffer))
    }

    /// Given a location, read a big endian value
    #[inline(always)]
    pub(super) fn read_be_value_internal<T: FromBytes>(
        &self,
        address: Address,
        current_timestamp: Period,
        members: &Members,
        avoid_side_effects: bool,
    ) -> Result<T, MemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.read_internal(
            address,
            current_timestamp,
            members,
            avoid_side_effects,
            buffer.as_mut(),
        )?;
        Ok(T::from_be_bytes(&buffer))
    }

    /// Step through the memory translation table to fill a buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn read(
        &self,
        address: Address,
        current_timestamp: Period,
        cache: Option<&mut AddressSpaceCache>,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.read_internal(address, current_timestamp, members, false, buffer)
        } else {
            let members = self.members.load();
            self.read_internal(address, current_timestamp, &members, false, buffer)
        }
    }

    #[inline]
    pub fn read_pure(
        &self,
        address: Address,
        current_timestamp: Period,
        cache: Option<&mut AddressSpaceCache>,
        buffer: &mut [u8],
    ) -> Result<(), MemoryError> {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.read_internal(address, current_timestamp, members, true, buffer)
        } else {
            let members = self.members.load();
            self.read_internal(address, current_timestamp, &members, true, buffer)
        }
    }

    /// Given a location, read a little endian value
    #[inline]
    pub fn read_le_value<T: FromBytes>(
        &self,
        address: Address,
        current_timestamp: Period,
        cache: Option<&mut AddressSpaceCache>,
    ) -> Result<T, MemoryError>
    where
        T::Bytes: Default,
    {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.read_le_value_internal(address, current_timestamp, members, false)
        } else {
            let members = self.members.load();
            self.read_le_value_internal(address, current_timestamp, &members, false)
        }
    }

    /// Given a location, read a little endian value
    #[inline]
    pub fn read_le_value_pure<T: FromBytes>(
        &self,
        address: Address,
        current_timestamp: Period,
        cache: Option<&mut AddressSpaceCache>,
    ) -> Result<T, MemoryError>
    where
        T::Bytes: Default,
    {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.read_le_value_internal(address, current_timestamp, members, true)
        } else {
            let members = self.members.load();
            self.read_le_value_internal(address, current_timestamp, &members, true)
        }
    }

    /// Given a location, read a big endian value
    #[inline]
    pub fn read_be_value<T: FromBytes>(
        &self,
        address: Address,
        current_timestamp: Period,
        cache: Option<&mut AddressSpaceCache>,
    ) -> Result<T, MemoryError>
    where
        T::Bytes: Default,
    {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.read_be_value_internal(address, current_timestamp, members, false)
        } else {
            let members = self.members.load();
            self.read_be_value_internal(address, current_timestamp, &members, false)
        }
    }

    /// Given a location, read a big endian value
    #[inline]
    pub fn read_be_value_pure<T: FromBytes>(
        &self,
        address: Address,
        current_timestamp: Period,
        cache: Option<&mut AddressSpaceCache>,
    ) -> Result<T, MemoryError>
    where
        T::Bytes: Default,
    {
        if let Some(cache) = cache {
            let members = cache.members.load();
            self.read_be_value_internal(address, current_timestamp, members, true)
        } else {
            let members = self.members.load();
            self.read_be_value_internal(address, current_timestamp, &members, true)
        }
    }
}
