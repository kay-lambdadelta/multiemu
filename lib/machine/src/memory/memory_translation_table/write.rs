use super::{
    MemoryHandle, MemoryTranslationTable, NeededAccess, RemapCallback,
    address_space::{AddressSpace, AddressSpaceHandle},
};
use crate::memory::{Address, callbacks::WriteMemory};
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
    pub fn insert_write_memory<M: WriteMemory>(&self, memory: M) -> MemoryHandle {
        self.memory_store.insert_write_memory(memory)
    }

    pub fn remap_write_memory(
        &self,
        handle: MemoryHandle,
        address_space: AddressSpaceHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();
        let address_space = address_spaces_guard.get_mut(address_space.get()).unwrap();

        assert!(
            self.memory_store.is_write_memory(handle),
            "Memory referred by handle does not have write capabilities"
        );

        address_space.remap_write_memory(handle, mapping);
    }

    /// Step through the memory translation table to give a set of components the buffer
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn write(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryOperationError> {
        let mut needed_accesses = Vec::from_iter([NeededAccess {
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
                            WriteMemoryOperationError(RangeInclusiveMap::from_iter([(
                                accessing_range.clone(),
                                WriteMemoryOperationErrorFailureType::OutOfBus,
                            )]))
                        })?;

                self.write_helper(
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
                    return Err(WriteMemoryOperationError(RangeInclusiveMap::from_iter([(
                        accessing_range,
                        WriteMemoryOperationErrorFailureType::OutOfBus,
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
    fn write_helper(
        &self,
        buffer: &[u8],
        did_handle: &mut bool,
        address: Address,
        address_space: AddressSpaceHandle,
        address_space_info: &AddressSpace,
        buffer_subrange: RangeInclusive<Address>,
        needed_accesses: &mut Vec<NeededAccess>,
        remap_callbacks: &mut Vec<RemapCallback>,
    ) -> Result<(), WriteMemoryOperationError> {
        // Cut off address
        let address = address & address_space_info.width_mask;

        tracing::debug!(
            "Writing to address {:#04x} from address space {:?}",
            address,
            address_space
        );

        let accessing_range =
            (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);

        for (component_assignment_range, memory_handle) in address_space_info
            .write_members
            .overlapping(accessing_range.clone())
        {
            self.memory_store.interact_write(*memory_handle, |memory| {
                *did_handle = true;

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                if let Err(errors) = memory.write_memory(
                    *overlap_start,
                    address_space,
                    &buffer[buffer_subrange.clone()],
                ) {
                    let mut detected_errors = RangeInclusiveMap::default();

                    for (range, error) in errors.records {
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
                        return Err(WriteMemoryOperationError(detected_errors));
                    }
                }
                Ok(())
            })?;
        }

        Ok(())
    }

    #[inline]
    pub fn write_le_value<T: ToBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        value: T,
    ) -> Result<(), WriteMemoryOperationError> {
        self.write(address, address_space, value.to_le_bytes().as_ref())
    }

    #[inline]
    pub fn write_be_value<T: ToBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        value: T,
    ) -> Result<(), WriteMemoryOperationError> {
        self.write(address, address_space, value.to_be_bytes().as_ref())
    }
}
