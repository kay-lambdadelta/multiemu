use super::{
    MemoryHandle, MemoryTranslationTable, NEEDED_ACCESSES_BASE_CAPACITY, NeededAccess,
    RemapCallback,
    address_space::{AddressSpace, AddressSpaceHandle},
};
use crate::memory::{Address, callbacks::ReadMemory};
use num::traits::FromBytes;
use rangemap::RangeInclusiveMap;
use smallvec::SmallVec;
use std::ops::RangeInclusive;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReadMemoryOperationErrorFailureType {
    Denied,
    OutOfBus,
}

#[derive(Error, Debug)]
#[error("Read operation failed: {0:#?}")]
pub struct ReadMemoryOperationError(
    RangeInclusiveMap<Address, ReadMemoryOperationErrorFailureType>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadMemoryRecord {
    /// Memory could not be read
    Denied,
    /// Memory redirects somewhere else
    Redirect {
        address: Address,
        address_space: AddressSpaceHandle,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum PreviewMemoryOperationErrorFailureType {
    Denied,
    OutOfBus,
    Impossible,
}

#[derive(Error, Debug)]
#[error("Preview operation failed (this really shouldn't be thrown): {0:#?}")]
pub struct PreviewMemoryOperationError(
    RangeInclusiveMap<Address, PreviewMemoryOperationErrorFailureType>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreviewMemoryRecord {
    /// Memory denied
    Denied,
    /// Memory redirects somewhere else
    Redirect {
        address: Address,
        address_space: AddressSpaceHandle,
    },
    // Memory here can't be read without an intense calculation or a state change
    Impossible,
}

impl MemoryTranslationTable {
    pub fn insert_read_memory<M: ReadMemory>(&self, memory: M) -> MemoryHandle {
        self.memory_store.insert_read_memory(memory)
    }

    pub fn remap_read_memory(
        &self,
        handle: MemoryHandle,
        address_space: AddressSpaceHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();
        let address_space = address_spaces_guard.get_mut(address_space.get()).unwrap();

        assert!(
            self.memory_store.is_read_memory(handle),
            "Memory referred by handle does not have read capabilities"
        );

        address_space.remap_read_memory(handle, mapping);
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
        let mut needed_accesses = SmallVec::from_iter([NeededAccess {
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
            }) = needed_accesses.pop()
            {
                let mut did_handle = false;

                let accessing_range =
                    (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);

                let address_spaces_guard = self.address_spaces.read().unwrap();
                let address_space_info =
                    address_spaces_guard
                        .get(address_space.get())
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
                    &mut needed_accesses,
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
        needed_accesses: &mut SmallVec<NeededAccess, NEEDED_ACCESSES_BASE_CAPACITY>,
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

        for (component_assignment_range, memory_handle) in address_space_info
            .write_members
            .overlapping(accessing_range.clone())
        {
            self.memory_store.interact_read(*memory_handle, |memory| {
                *did_handle = true;

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                if let Err(errors) = memory.read_memory(
                    *overlap_start,
                    address_space,
                    &mut buffer[buffer_subrange.clone()],
                ) {
                    let mut detected_errors = RangeInclusiveMap::default();

                    for (range, error) in errors.records {
                        match error {
                            ReadMemoryRecord::Denied => {
                                tracing::debug!("Write memory operation denied at {:#04x?}", range);

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
            })?;
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
        let mut needed_accesses = SmallVec::from_iter([NeededAccess {
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
            let address_space_info =
                address_spaces_guard
                    .get(address_space.get())
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
        needed_accesses: &mut SmallVec<NeededAccess, NEEDED_ACCESSES_BASE_CAPACITY>,
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

        for (component_assignment_range, memory_handle) in address_space_info
            .write_members
            .overlapping(accessing_range.clone())
        {
            self.memory_store.interact_read(*memory_handle, |memory| {
                *did_handle = true;

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                if let Err(errors) = memory.preview_memory(
                    *overlap_start,
                    address_space,
                    &mut buffer[buffer_subrange.clone()],
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
            })?;
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
