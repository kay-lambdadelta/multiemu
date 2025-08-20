use super::{MemoryAccessTable, address_space::AddressSpaceHandle};
use crate::memory::{Address, table::QueueEntry};
use num::traits::ToBytes;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use smallvec::SmallVec;
use std::{hash::Hash, ops::RangeInclusive};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Why a write operation failed
pub enum WriteMemoryOperationErrorFailureType {
    /// Access was denied
    Denied,
    /// Nothing is mapped there
    OutOfBus,
}

#[derive(Error, Debug)]
#[error("Write operation failed: {0:#?}")]
/// Wrapper around the error type in order to specific ranges
pub struct WriteMemoryOperationError(
    RangeInclusiveMap<Address, WriteMemoryOperationErrorFailureType>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Why a write operation from a component failed
pub enum WriteMemoryRecord {
    /// Memory could not be written
    Denied,
    /// Memory redirects somewhere else
    Redirect {
        /// The address it redirects to
        address: Address,
        /// The address space it redirects to
        address_space: AddressSpaceHandle,
    },
}

impl MemoryAccessTable {
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
                WriteMemoryOperationError(RangeInclusiveMap::from_iter([(
                    (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                    WriteMemoryOperationErrorFailureType::OutOfBus,
                )]))
            })?;

            // TODO: Handle width mask wraparound properly
            let accessing_range = (buffer_subrange.start() + address)
                & address_space_info.width_mask
                ..=(buffer_subrange.end() + address) & address_space_info.width_mask;
            let address = address & address_space_info.width_mask;

            address_space_info.visit_write_components(
                accessing_range.clone(),
                |component_id, component_assigned_range| {
                    self.registry
                    .interact_dyn(component_id, |component| {
                        let adjusted_accessing_range: RangeInclusive<Address> = accessing_range
                            .clone()
                            .intersection(component_assigned_range.clone())
                            .into();

                        let adjusted_buffer_subrange = (adjusted_accessing_range.start() - address)
                            ..=(adjusted_accessing_range.end() - address);

                        did_handle = true;

                        if let Err(errors) = component.write_memory(
                            *adjusted_accessing_range.start(),
                            address_space,
                            &buffer[adjusted_buffer_subrange.clone()],
                        ) {
                            let mut detected_errors = RangeInclusiveMap::default();

                            for (range, error) in errors.records {
                                match error {
                                    WriteMemoryRecord::Denied => {
                                        detected_errors.insert(
                                            range,
                                            WriteMemoryOperationErrorFailureType::Denied,
                                        );
                                    }
                                    WriteMemoryRecord::Redirect {
                                        address: redirect_address,
                                        address_space: redirect_address_space,
                                    } => {
                                        assert!(
                                            !component_assigned_range.contains(&redirect_address)
                                                && address_space == redirect_address_space,
                                            "Memory attempted to redirect to itself {:x?} -> {:x}",
                                            component_assigned_range,
                                            redirect_address,
                                        );

                                        queue.push(QueueEntry {
                                            address: redirect_address,
                                            address_space: redirect_address_space,
                                            buffer_subrange: (range.start() - address)
                                                ..=(range.end() - address),
                                        });
                                    }
                                }
                            }

                            if !detected_errors.is_empty() {
                                return Err(WriteMemoryOperationError(detected_errors));
                            }
                        }
                        Ok(())
                    })
                    .unwrap()?;

                    Ok(())
                },
            )?;
        }

        if !did_handle {
            return Err(WriteMemoryOperationError(RangeInclusiveMap::from_iter([(
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                WriteMemoryOperationErrorFailureType::OutOfBus,
            )])));
        }

        Ok(())
    }

    #[inline]
    /// Helper function to write with a little endian value
    pub fn write_le_value<T: ToBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        value: T,
    ) -> Result<(), WriteMemoryOperationError> {
        self.write(address, address_space, value.to_le_bytes().as_ref())
    }

    #[inline]
    /// Helper function to write with a big endian value
    pub fn write_be_value<T: ToBytes>(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        value: T,
    ) -> Result<(), WriteMemoryOperationError> {
        self.write(address, address_space, value.to_be_bytes().as_ref())
    }
}
