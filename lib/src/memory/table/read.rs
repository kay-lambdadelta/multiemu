use super::{MemoryAccessTable, address_space::AddressSpaceId};
use crate::{
    component::Component,
    memory::{Address, table::QueueEntry},
};
use num::traits::FromBytes;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use smallvec::SmallVec;
use std::{hash::Hash, ops::RangeInclusive};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Why a read operation failed
pub enum ReadMemoryOperationErrorFailureType {
    /// Access was denied
    Denied,
    /// Nothing is mapped there
    OutOfBus,
}

#[derive(Error, Debug)]
#[error("Read operation failed: {0:#x?}")]
/// Wrapper around the error type in order to specify ranges
pub struct ReadMemoryOperationError(
    RangeInclusiveMap<Address, ReadMemoryOperationErrorFailureType>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Why a read operation from a component failed
pub enum ReadMemoryRecord {
    /// Memory could not be read
    Denied,
    /// Memory redirects somewhere else
    Redirect {
        /// Address
        address: Address,
        /// Address Space
        address_space: AddressSpaceId,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Why a preview operation failed
pub enum PreviewMemoryOperationErrorFailureType {
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
pub struct PreviewMemoryOperationError(
    RangeInclusiveMap<Address, PreviewMemoryOperationErrorFailureType>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Why a preview operation from a component failed
pub enum PreviewMemoryRecord {
    /// Memory denied
    Denied,
    /// Memory redirects somewhere else
    Redirect {
        /// Address
        address: Address,
        /// Address space
        address_space: AddressSpaceId,
    },
    // Memory here can't be read without an intense calculation or a state change
    Impossible,
}

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
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryOperationError> {
        let buffer_subrange = 0..=(buffer.len() - 1);
        let mut queue = SmallVec::<[QueueEntry; 1]>::from([QueueEntry {
            address,
            address_space,
            buffer_subrange: buffer_subrange.clone(),
        }]);

        let mut did_handle = false;

        while let Some(QueueEntry {
            address,
            address_space,
            buffer_subrange,
        }) = queue.pop()
        {
            let address_space_info = self.address_spaces.get(&address_space).ok_or_else(|| {
                ReadMemoryOperationError(RangeInclusiveMap::from_iter([(
                    (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                    ReadMemoryOperationErrorFailureType::OutOfBus,
                )]))
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

            members.visit_read::<ReadMemoryOperationError>(
                access_range.clone(),
                |component_assigned_range, component| {
                    did_handle = true;

                    let access_range: RangeInclusive<_> = component_assigned_range
                        .clone()
                        .intersection(access_range.clone())
                        .into();

                    let buffer_range =
                        (access_range.start() - address)..=(access_range.end() - address);

                    self.registry
                        .get()
                        .unwrap()
                        .interact_dyn(component, |component| {
                            read_helper(
                                buffer,
                                &mut queue,
                                address,
                                access_range,
                                buffer_range,
                                component,
                                address_space,
                            )?;

                            Ok(())
                        })
                        .unwrap()
                },
            )?;
        }

        if !did_handle {
            return Err(ReadMemoryOperationError(RangeInclusiveMap::from_iter([(
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                ReadMemoryOperationErrorFailureType::OutOfBus,
            )])));
        }

        Ok(())
    }

    #[inline]
    pub fn read_le_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
    ) -> Result<T, ReadMemoryOperationError>
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
    ) -> Result<T, ReadMemoryOperationError>
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
    ) -> Result<(), PreviewMemoryOperationError> {
        let buffer_subrange = 0..=(buffer.len() - 1);
        let mut queue = SmallVec::<[QueueEntry; 1]>::from([QueueEntry {
            address,
            address_space,
            buffer_subrange: buffer_subrange.clone(),
        }]);

        let mut did_handle = false;

        while let Some(QueueEntry {
            address,
            address_space,
            buffer_subrange,
        }) = queue.pop()
        {
            let address_space_info = self.address_spaces.get(&address_space).ok_or_else(|| {
                PreviewMemoryOperationError(RangeInclusiveMap::from_iter([(
                    (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                    PreviewMemoryOperationErrorFailureType::OutOfBus,
                )]))
            })?;

            // TODO: Handle width mask wraparound properly
            let access_range = (buffer_subrange.start() + address) & address_space_info.width_mask
                ..=(buffer_subrange.end() + address) & address_space_info.width_mask;
            let address = address & address_space_info.width_mask;

            let members = address_space_info.get_members(
                self.registry
                    .get()
                    .expect("Cannot do preview until machine is finished building"),
            );

            members.visit_read::<PreviewMemoryOperationError>(
                access_range.clone(),
                |component_assigned_range, component| {
                    did_handle = true;

                    let access_range: RangeInclusive<_> = component_assigned_range
                        .clone()
                        .intersection(access_range.clone())
                        .into();

                    let buffer_range =
                        (access_range.start() - address)..=(access_range.end() - address);

                    self.registry
                        .get()
                        .unwrap()
                        .interact_dyn(component, |component| {
                            preview_helper(
                                buffer,
                                &mut queue,
                                address,
                                access_range,
                                buffer_range,
                                component,
                                address_space,
                            )?;

                            Ok(())
                        })
                        .unwrap()
                },
            )?;
        }

        if !did_handle {
            return Err(PreviewMemoryOperationError(RangeInclusiveMap::from_iter([
                (
                    (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                    PreviewMemoryOperationErrorFailureType::OutOfBus,
                ),
            ])));
        }

        Ok(())
    }

    #[inline]
    pub fn preview_le_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceId,
    ) -> Result<T, PreviewMemoryOperationError>
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
    ) -> Result<T, PreviewMemoryOperationError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.preview(address, address_space, buffer.as_mut())?;
        Ok(T::from_be_bytes(&buffer))
    }
}

#[inline]
fn read_helper(
    buffer: &mut [u8],
    queue: &mut SmallVec<[QueueEntry; 1]>,
    address: usize,
    access_range: RangeInclusive<usize>,
    buffer_range: RangeInclusive<usize>,
    component: &dyn Component,
    address_space: AddressSpaceId,
) -> Result<(), ReadMemoryOperationError> {
    if let Err(errors) = component.read_memory(
        *access_range.start(),
        address_space,
        &mut buffer[buffer_range.clone()],
    ) {
        let mut detected_errors = RangeInclusiveMap::default();

        for (range, error) in errors.records {
            match error {
                ReadMemoryRecord::Denied => {
                    detected_errors.insert(range, ReadMemoryOperationErrorFailureType::Denied);
                }
                ReadMemoryRecord::Redirect {
                    address: redirect_address,
                    address_space: redirect_address_space,
                } => {
                    debug_assert!(
                        !(access_range.contains(&redirect_address)
                            && address_space == redirect_address_space),
                        "Memory attempted to redirect to itself {:x?} -> {:x}",
                        access_range,
                        redirect_address,
                    );

                    queue.push(QueueEntry {
                        address: redirect_address,
                        address_space: redirect_address_space,
                        buffer_subrange: (range.start() - address)..=(range.end() - address),
                    });
                }
            }
        }

        if !detected_errors.is_empty() {
            return Err(ReadMemoryOperationError(detected_errors));
        }
    }

    Ok(())
}

#[inline]
fn preview_helper(
    buffer: &mut [u8],
    queue: &mut SmallVec<[QueueEntry; 1]>,
    address: usize,
    access_range: RangeInclusive<usize>,
    buffer_range: RangeInclusive<usize>,
    component: &dyn Component,
    address_space: AddressSpaceId,
) -> Result<(), PreviewMemoryOperationError> {
    if let Err(errors) = component.preview_memory(
        *access_range.start(),
        address_space,
        &mut buffer[buffer_range.clone()],
    ) {
        let mut detected_errors = RangeInclusiveMap::default();

        for (range, error) in errors.records {
            match error {
                PreviewMemoryRecord::Denied => {
                    detected_errors.insert(range, PreviewMemoryOperationErrorFailureType::Denied);
                }
                PreviewMemoryRecord::Redirect {
                    address: redirect_address,
                    address_space: redirect_address_space,
                } => {
                    debug_assert!(
                        !(access_range.contains(&redirect_address)
                            && address_space == redirect_address_space),
                        "Memory attempted to redirect to itself {:x?} -> {:x}",
                        access_range,
                        redirect_address,
                    );

                    queue.push(QueueEntry {
                        address: redirect_address,
                        address_space: redirect_address_space,
                        buffer_subrange: (range.start() - address)..=(range.end() - address),
                    });
                }
                PreviewMemoryRecord::Impossible => {
                    detected_errors
                        .insert(range, PreviewMemoryOperationErrorFailureType::Impossible);
                }
            }
        }

        if !detected_errors.is_empty() {
            return Err(PreviewMemoryOperationError(detected_errors));
        }
    }

    Ok(())
}
