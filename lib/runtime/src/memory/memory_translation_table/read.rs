use super::{
    MemoryTranslationTable, NeededAccess, RemapCallback,
    address_space::{AddressSpace, AddressSpaceHandle},
};
use crate::{component::ComponentId, memory::Address};
use num::traits::FromBytes;
use rangemap::RangeInclusiveMap;
use std::{ops::RangeInclusive, vec::Vec};
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
#[error("Read operation failed: {0:#?}")]
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
        address_space: AddressSpaceHandle,
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
#[error("Preview operation failed (if you see this in a panic this is a bug): {0:#?}")]
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
        address_space: AddressSpaceHandle,
    },
    // Memory here can't be read without an intense calculation or a state change
    Impossible,
}

impl MemoryTranslationTable {
    pub fn remap_read_memory(
        &self,
        component_id: ComponentId,
        address_space: AddressSpaceHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();
        let address_space = address_spaces_guard
            .get_mut(address_space.get() as usize)
            .unwrap();

        address_space.remap_read_memory(component_id, mapping);
    }

    /// Step through the memory translation table to fill a buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn read(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryOperationError> {
        let mut needed_access = Vec::from_iter([NeededAccess {
            address,
            address_space,
            buffer_subrange: (0..=(buffer.len() - 1)),
        }]);
        let mut remap_callbacks = Vec::default();

        let result = (|| {
            while let Some(NeededAccess {
                address,
                address_space,
                buffer_subrange,
            }) = needed_access.pop()
            {
                let mut did_handle = false;

                let accessing_range =
                    (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);

                let address_spaces_guard = self.address_spaces.read().unwrap();
                let address_space_info = address_spaces_guard
                    .get(address_space.get() as usize)
                    .ok_or_else(|| {
                        ReadMemoryOperationError(RangeInclusiveMap::from_iter([(
                            accessing_range.clone(),
                            ReadMemoryOperationErrorFailureType::OutOfBus,
                        )]))
                    })?;

                self.read_helper(
                    buffer,
                    &mut did_handle,
                    address,
                    address_space,
                    address_space_info,
                    buffer_subrange,
                    &mut needed_access,
                    &mut remap_callbacks,
                )?;

                if !did_handle {
                    return Err(ReadMemoryOperationError(RangeInclusiveMap::from_iter([(
                        accessing_range,
                        ReadMemoryOperationErrorFailureType::OutOfBus,
                    )])));
                }
            }

            Ok(())
        })();

        self.process_remap_callbacks(remap_callbacks);

        result
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn read_helper(
        &self,
        buffer: &mut [u8],
        did_handle: &mut bool,
        address: Address,
        address_space: AddressSpaceHandle,
        address_space_info: &AddressSpace,
        buffer_subrange: RangeInclusive<Address>,
        needed_accesses: &mut Vec<NeededAccess>,
        remap_callbacks: &mut Vec<RemapCallback>,
    ) -> Result<(), ReadMemoryOperationError> {
        // Cut off address
        let address = address & address_space_info.width_mask;

        tracing::debug!(
            "Reading from address {:#04x} from address space {:?}",
            address,
            address_space
        );

        let accessing_range =
            (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);

        for (component_assignment_range, component_id) in address_space_info
            .read_members
            .overlapping(accessing_range.clone())
        {
            let adjusted_accessing_range = (*accessing_range
                .start()
                .max(component_assignment_range.start()))
                ..=(*accessing_range.end().min(component_assignment_range.end()));

            let adjusted_buffer_subrange = (adjusted_accessing_range.start() - address)
                ..=(adjusted_accessing_range.end() - address);

            self.component_store.interact_dyn(*component_id, |component| {
                *did_handle = true;

                if let Err(errors) = component.read_memory(
                    *adjusted_accessing_range.start(),
                    address_space,
                    &mut buffer[adjusted_buffer_subrange.clone()],
                ) {
                    let mut detected_errors = RangeInclusiveMap::default();

                    for (range, error) in errors.records {
                        match error {
                            ReadMemoryRecord::Denied => {
                                tracing::debug!("Read memory operation denied at {:#04x?}", range);

                                detected_errors
                                    .insert(range, ReadMemoryOperationErrorFailureType::Denied);
                            }
                            ReadMemoryRecord::Redirect {
                                address: redirect_address,
                                address_space: redirect_address_space,
                            } => {
                                assert!(
                                    !component_assignment_range.contains(&redirect_address)
                                        && address_space == redirect_address_space,
                                    "Memory attempted to redirect to itself {:x?} -> {:x}",
                                    component_assignment_range,
                                    redirect_address,
                                );

                                tracing::debug!(
                                    "Read memory operation redirected from {:#04x?} to {:#04x?} in address space {:?}",
                                    range,
                                    redirect_address,
                                    redirect_address_space
                                );

                                needed_accesses.push(NeededAccess {
                                    address: redirect_address,
                                    address_space: redirect_address_space,
                                    buffer_subrange: (range.start() - address)
                                        ..=(range.end() - address),
                                });
                            }
                        }
                    }

                    remap_callbacks.extend(errors.remap_callback);

                    if !detected_errors.is_empty() {
                        return Err(ReadMemoryOperationError(detected_errors));
                    }
                }
                Ok(())
            }).unwrap()?;
        }

        Ok(())
    }

    #[inline]
    pub fn read_le_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
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
        address_space: AddressSpaceHandle,
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
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), PreviewMemoryOperationError> {
        let mut needed_accesses = Vec::from_iter([NeededAccess {
            address,
            address_space,
            buffer_subrange: (0..=(buffer.len() - 1)),
        }]);

        while let Some(NeededAccess {
            address,
            address_space,
            buffer_subrange,
        }) = needed_accesses.pop()
        {
            let mut did_handle = false;

            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);

            let address_spaces_guard = self.address_spaces.read().unwrap();
            let address_space_info = address_spaces_guard
                .get(address_space.get() as usize)
                .ok_or_else(|| {
                    PreviewMemoryOperationError(RangeInclusiveMap::from_iter([(
                        accessing_range.clone(),
                        PreviewMemoryOperationErrorFailureType::OutOfBus,
                    )]))
                })?;

            self.preview_helper(
                buffer,
                &mut did_handle,
                address,
                address_space,
                address_space_info,
                buffer_subrange,
                &mut needed_accesses,
            )?;

            if !did_handle {
                return Err(PreviewMemoryOperationError(RangeInclusiveMap::from_iter([
                    (
                        accessing_range,
                        PreviewMemoryOperationErrorFailureType::OutOfBus,
                    ),
                ])));
            }
        }

        Ok(())
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn preview_helper(
        &self,
        buffer: &mut [u8],
        did_handle: &mut bool,
        address: Address,
        address_space: AddressSpaceHandle,
        address_space_info: &AddressSpace,
        buffer_subrange: RangeInclusive<Address>,
        needed_accesses: &mut Vec<NeededAccess>,
    ) -> Result<(), PreviewMemoryOperationError> {
        // Cut off address
        let address = address & address_space_info.width_mask;

        tracing::debug!(
            "Previewing from address {:#04x} from address space {:?}",
            address,
            address_space
        );

        let accessing_range =
            (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);

        for (component_assignment_range, component_id) in address_space_info
            .read_members
            .overlapping(accessing_range.clone())
        {
            let adjusted_accessing_range = (*accessing_range
                .start()
                .max(component_assignment_range.start()))
                ..=(*accessing_range.end().min(component_assignment_range.end()));

            let adjusted_buffer_subrange = (adjusted_accessing_range.start() - address)
                ..=(adjusted_accessing_range.end() - address);

            self.component_store.interact_dyn(*component_id, |component| {
                *did_handle = true;

                if let Err(errors) = component.preview_memory(
                    *adjusted_accessing_range.start(),
                    address_space,
                    &mut buffer[adjusted_buffer_subrange.clone()],
                ) {
                    let mut detected_errors = RangeInclusiveMap::default();

                    for (range, error) in errors.records {
                        match error {
                            PreviewMemoryRecord::Denied => {
                                tracing::debug!(
                                    "Preview memory operation denied at {:#04x?}",
                                    range
                                );

                                detected_errors
                                    .insert(range, PreviewMemoryOperationErrorFailureType::Denied);
                            }
                            PreviewMemoryRecord::Redirect {
                                address: redirect_address,
                                address_space: redirect_address_space,
                            } => {
                                assert!(
                                    !component_assignment_range.contains(&redirect_address)
                                        && address_space == redirect_address_space,
                                    "Memory attempted to redirect to itself {:x?} -> {:x}",
                                    component_assignment_range,
                                    redirect_address,
                                );

                                tracing::debug!(
                                    "Preview memory operation redirected from {:#04x?} to {:#04x?} in address space {:?}",
                                    range,
                                    redirect_address,
                                    redirect_address_space
                                );

                                needed_accesses.push(NeededAccess {
                                    address: redirect_address,
                                    address_space: redirect_address_space,
                                    buffer_subrange: (range.start() - address)
                                        ..=(range.end() - address),
                                });
                            }
                            PreviewMemoryRecord::Impossible => {
                                tracing::debug!(
                                    "Preview memory operation impossible at {:#04x?}",
                                    range
                                );

                                detected_errors
                                    .insert(range, PreviewMemoryOperationErrorFailureType::Denied);
                            },
                        }
                    }

                    if errors.remap_callback.is_some() {
                        panic!("Cannot remap preview memory operation");
                    }

                    if !detected_errors.is_empty() {
                        return Err(PreviewMemoryOperationError(detected_errors));
                    }
                }

                Ok(())
            }).unwrap()?;
        }

        Ok(())
    }

    #[inline]
    pub fn preview_le_value<T: FromBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
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
        address_space: AddressSpaceHandle,
    ) -> Result<T, PreviewMemoryOperationError>
    where
        T::Bytes: Default,
    {
        let mut buffer = T::Bytes::default();
        self.preview(address, address_space, buffer.as_mut())?;
        Ok(T::from_be_bytes(&buffer))
    }
}
