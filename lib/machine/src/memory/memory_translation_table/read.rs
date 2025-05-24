use super::{MemoryHandle, MemoryTranslationTable, StoredCallback};
use crate::memory::{Address, AddressSpaceHandle, callbacks::ReadMemory};
use num::traits::FromBytes;
use rangemap::RangeInclusiveMap;
use std::ops::RangeInclusive;
use thiserror::Error;

pub struct ReadMemoryErrorAccumulator();

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
    pub fn insert_read_memory<M: ReadMemory>(
        &self,
        memory: M,
        mappings: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) -> MemoryHandle {
        let mut impl_guard = self.0.write().unwrap();

        let id = MemoryHandle::new(impl_guard.current_memory_id);
        impl_guard.current_memory_id = impl_guard
            .current_memory_id
            .checked_add(1)
            .expect("Too many memories");

        impl_guard
            .memories
            .insert(id, StoredCallback::Read(Box::new(memory)));

        for (address_space, addresses) in mappings {
            let address_spaces = impl_guard
                .address_spaces
                .get_mut(&address_space)
                .expect("Non existant address space");
            address_spaces.read_members.insert(addresses, id);
        }

        id
    }

    /// Step through the memory translation table to fill the buffer with data
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn read(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryOperationError> {
        let impl_guard = self.0.read().unwrap();

        let mut needed_accesses =
            Vec::from_iter([(address, address_space, (0..=buffer.len() - 1))]);

        while let Some((address, address_space, buffer_subrange)) = needed_accesses.pop() {
            let address_space_info = impl_guard
                .address_spaces
                .get(&address_space)
                .expect("Non existant address space");

            // Cut off address
            let address = address & address_space_info.width_mask;

            tracing::debug!(
                "Reading from address {:#04x} from address space {:?}",
                address,
                address_space
            );

            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, memory_id) in address_space_info
                .read_members
                .overlapping(accessing_range.clone())
            {
                let memory = impl_guard
                    .memories
                    .get(memory_id)
                    .and_then(|memory| match memory {
                        StoredCallback::Read(memory) => Some(memory.as_ref()),
                        StoredCallback::ReadWrite(memory) => {
                            Some(memory.as_ref() as &dyn ReadMemory)
                        }
                        _ => None,
                    })
                    .expect("Non existant memory");

                did_handle = true;
                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                if let Err(errors) = memory.read_memory(
                    *overlap_start,
                    address_space,
                    &mut buffer[buffer_subrange.clone()],
                ) {
                    let mut detected_errors = RangeInclusiveMap::default();

                    if !detected_errors.is_empty() {
                        return Err(ReadMemoryOperationError(detected_errors));
                    }

                    for (range, error) in errors {
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

                                needed_accesses.push((
                                    redirect_address,
                                    redirect_address_space,
                                    (range.start() - address)..=(range.end() - address),
                                ));
                            }
                        }
                    }
                }
            }

            if !did_handle {
                return Err(ReadMemoryOperationError(RangeInclusiveMap::from_iter([(
                    accessing_range,
                    ReadMemoryOperationErrorFailureType::OutOfBus,
                )])));
            }
        }

        Ok(())
    }

    #[inline]
    pub fn read_le_value<T: FromBytes>(
        &self,
        address: usize,
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
        address: usize,
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
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), PreviewMemoryOperationError> {
        let impl_guard = self.0.write().unwrap();

        let mut needed_accesses =
            Vec::from_iter([(address, address_space, (0..=(buffer.len() - 1)))]);

        while let Some((address, address_space, buffer_subrange)) = needed_accesses.pop() {
            let address_space_info = impl_guard
                .address_spaces
                .get(&address_space)
                .expect("Non existant address space");

            let address = address & address_space_info.width_mask;

            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, memory_id) in address_space_info
                .read_members
                .overlapping(accessing_range.clone())
            {
                let memory = impl_guard
                    .memories
                    .get(memory_id)
                    .and_then(|memory| match memory {
                        StoredCallback::Read(memory) => Some(memory.as_ref()),
                        StoredCallback::ReadWrite(memory) => {
                            Some(memory.as_ref() as &dyn ReadMemory)
                        }
                        _ => None,
                    })
                    .expect("Non existant memory");

                did_handle = true;
                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                if let Err(errors) = memory.preview_memory(
                    *overlap_start,
                    address_space,
                    &mut buffer[buffer_subrange.clone()],
                ) {
                    let mut detected_errors = RangeInclusiveMap::default();

                    for (range, error) in errors {
                        match error {
                            PreviewMemoryRecord::Denied => {
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

                                needed_accesses.push((
                                    redirect_address,
                                    redirect_address_space,
                                    (range.start() - address)..=(range.end() - address),
                                ));
                            }
                            PreviewMemoryRecord::Impossible => {
                                detected_errors.insert(
                                    range,
                                    PreviewMemoryOperationErrorFailureType::Impossible,
                                );
                            }
                        }
                    }

                    if !detected_errors.is_empty() {
                        return Err(PreviewMemoryOperationError(detected_errors));
                    }
                }
            }

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
    pub fn preview_le_value<T: FromBytes>(
        &self,
        address: usize,
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
        address: usize,
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
