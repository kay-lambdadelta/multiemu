use super::{MemoryHandle, MemoryTranslationTable, StoredCallback};
use crate::memory::{
    Address, AddressSpaceHandle, VALID_MEMORY_ACCESS_SIZES, callbacks::WriteMemory,
};
use num::traits::ToBytes;
use rangemap::RangeInclusiveMap;
use std::ops::RangeInclusive;
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WriteMemoryOperationErrorFailureType {
    Denied,
    OutOfBus,
}

#[derive(Error, Debug)]
#[error("Write operation failed: {0:#?}")]
pub struct WriteMemoryOperationError(
    RangeInclusiveMap<Address, WriteMemoryOperationErrorFailureType>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WriteMemoryRecord {
    /// Memory could not be written
    Denied,
    /// Memory redirects somewhere else
    Redirect {
        address: Address,
        address_space: AddressSpaceHandle,
    },
}

impl MemoryTranslationTable {
    pub fn insert_write_memory<M: WriteMemory>(
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
            .insert(id, StoredCallback::Write(Box::new(memory)));

        for (address_space, addresses) in mappings {
            let address_spaces = impl_guard
                .address_spaces
                .get_mut(&address_space)
                .expect("Non existant address space");
            address_spaces.write_members.insert(addresses, id);
        }

        id
    }

    /// Step through the memory translation table to give a set of components the buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn write(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryOperationError> {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );
        let impl_guard = self.0.write().unwrap();

        let mut needed_accesses =
            Vec::from_iter([(address, address_space, (0..=(buffer.len() - 1)))]);

        while let Some((address, address_space, buffer_subrange)) = needed_accesses.pop() {
            let address_space_info = impl_guard
                .address_spaces
                .get(&address_space)
                .expect("Non existant address space");

            // Cut off address
            let address = address & address_space_info.width_mask;

            tracing::debug!(
                "Writing to address {:#04x} from address space {:?}",
                address,
                address_space
            );

            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, memory_id) in address_space_info
                .write_members
                .overlapping(accessing_range.clone())
            {
                let memory = impl_guard
                    .memories
                    .get(memory_id)
                    .and_then(|memory| match memory {
                        StoredCallback::Write(memory) => Some(memory.as_ref()),
                        StoredCallback::ReadWrite(memory) => {
                            Some(memory.as_ref() as &dyn WriteMemory)
                        }
                        _ => None,
                    })
                    .expect("Non existant memory");

                did_handle = true;

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                if let Err(errors) = memory.write_memory(
                    *overlap_start,
                    address_space,
                    &buffer[buffer_subrange.clone()],
                ) {
                    let mut detected_errors = RangeInclusiveMap::default();

                    for (range, error) in errors {
                        match error {
                            WriteMemoryRecord::Denied => {
                                tracing::debug!("Write memory operation denied at {:#04x?}", range);

                                detected_errors
                                    .insert(range, WriteMemoryOperationErrorFailureType::Denied);
                            }
                            WriteMemoryRecord::Redirect {
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
                                    "Write memory operation redirected from {:#04x?} to {:#04x?} in address space {:?}",
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

                    if !detected_errors.is_empty() {
                        return Err(WriteMemoryOperationError(detected_errors));
                    }
                }
            }

            if !did_handle {
                return Err(WriteMemoryOperationError(RangeInclusiveMap::from_iter([(
                    accessing_range,
                    WriteMemoryOperationErrorFailureType::OutOfBus,
                )])));
            }
        }

        Ok(())
    }

    #[inline]
    pub fn write_le_value<T: ToBytes>(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        value: T,
    ) -> Result<(), WriteMemoryOperationError> {
        self.write(address, address_space, value.to_le_bytes().as_ref())
    }

    #[inline]
    pub fn write_be_value<T: ToBytes>(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        value: T,
    ) -> Result<(), WriteMemoryOperationError> {
        self.write(address, address_space, value.to_be_bytes().as_ref())
    }
}
