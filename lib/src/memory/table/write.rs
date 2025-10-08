use super::{MemoryAccessTable, address_space::AddressSpaceId};
use crate::memory::Address;
use num::traits::ToBytes;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
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
    #[inline(always)]
    pub fn write(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryError> {
        let buffer_subrange = 0..=(buffer.len() - 1);
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

        // TODO: Handle width mask wraparound properly
        let access_range = (buffer_subrange.start() + address) & address_space_info.width_mask
            ..=(buffer_subrange.end() + address) & address_space_info.width_mask;
        let address = address & address_space_info.width_mask;

        let members = address_space_info.get_members(
            self.registry
                .get()
                .expect("Cannot do reads until machine is finished building"),
        );

        members.visit_write::<WriteMemoryError>(
            access_range.clone(),
            |component_assigned_range, component| {
                let access_range: RangeInclusive<_> = component_assigned_range
                    .clone()
                    .intersection(access_range.clone())
                    .into();

                let buffer_subrange =
                    (access_range.start() - address)..=(access_range.end() - address);

                self.registry
                    .get()
                    .unwrap()
                    .interact_dyn_mut(
                        component,
                        #[inline(always)]
                        |component| {
                            component.write_memory(
                                *access_range.start(),
                                address_space,
                                &buffer[buffer_subrange.clone()],
                            )?;

                            Ok(())
                        },
                    )
                    .unwrap()
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
