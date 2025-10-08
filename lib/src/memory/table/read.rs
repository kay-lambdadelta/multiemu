use super::{MemoryAccessTable, address_space::AddressSpaceId};
use crate::memory::Address;
use num::traits::FromBytes;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use std::ops::RangeInclusive;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Why a read operation failed
pub enum ReadMemoryErrorType {
    /// Access was denied
    Denied,
    /// Nothing is mapped there
    OutOfBus,
}

#[derive(Error, Debug)]
#[error("Read operation failed: {0:#x?}")]
/// Wrapper around the error type in order to specify ranges
pub struct ReadMemoryError(pub RangeInclusiveMap<Address, ReadMemoryErrorType>);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Why a preview operation failed
pub enum PreviewMemoryErrorType {
    /// Memory could not be read
    Denied,
    /// Nothing is mapped there
    OutOfBus,
    /// It would be impossible to view this memory without a state change
    Impossible,
}

#[derive(Error, Debug)]
#[error("Preview operation failed (if you see this in a panic this is a bug): {0:#x?}")]
/// Wrapper around the error type in order to specify ranges
pub struct PreviewMemoryError(pub RangeInclusiveMap<Address, PreviewMemoryErrorType>);

impl MemoryAccessTable {
    /// Step through the memory translation table to fill a buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    ///
    #[inline(always)]
    pub fn read(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryError> {
        let buffer_subrange = 0..=(buffer.len() - 1);
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

        // TODO: Handle width mask wraparound properly
        let access_range = (buffer_subrange.start() + address) & address_space_info.width_mask
            ..=(buffer_subrange.end() + address) & address_space_info.width_mask;
        let address = address & address_space_info.width_mask;

        let members = address_space_info.get_members(
            self.registry
                .get()
                .expect("Cannot do reads until machine is finished building"),
        );

        members.visit_read::<ReadMemoryError>(
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
                    .interact_dyn(
                        component,
                        #[inline(always)]
                        |component| {
                            component.read_memory(
                                *access_range.start(),
                                address_space,
                                &mut buffer[buffer_subrange.clone()],
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
    pub fn read_le_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
    ) -> Result<T, ReadMemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.read(address, address_space, buffer.as_mut())?;
        Ok(T::from_le_bytes(&buffer))
    }

    #[inline]
    pub fn read_be_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
    ) -> Result<T, ReadMemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.read(address, address_space, buffer.as_mut())?;
        Ok(T::from_be_bytes(&buffer))
    }

    #[inline]
    pub fn preview(
        &self,
        address: Address,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), PreviewMemoryError> {
        let buffer_subrange = 0..=(buffer.len() - 1);
        let address_space_info = self
            .address_spaces
            .get(address_space.0 as usize)
            .ok_or_else({
                let buffer_subrange = buffer_subrange.clone();

                move || {
                    PreviewMemoryError(RangeInclusiveMap::from_iter([(
                        (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                        PreviewMemoryErrorType::OutOfBus,
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

        members.visit_read::<PreviewMemoryError>(
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
                    .interact_dyn(
                        component,
                        #[inline(always)]
                        |component| {
                            component.preview_memory(
                                *access_range.start(),
                                address_space,
                                &mut buffer[buffer_subrange.clone()],
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
    pub fn preview_le_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
    ) -> Result<T, PreviewMemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.preview(address, address_space, buffer.as_mut())?;
        Ok(T::from_le_bytes(&buffer))
    }

    #[inline]
    pub fn preview_be_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
    ) -> Result<T, PreviewMemoryError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.preview(address, address_space, buffer.as_mut())?;
        Ok(T::from_be_bytes(&buffer))
    }
}
