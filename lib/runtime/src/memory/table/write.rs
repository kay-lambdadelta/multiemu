use super::{MemoryAccessTable, RemapCallback, address_space::AddressSpaceHandle};
use crate::{component::ComponentId, memory::Address};
use num::traits::ToBytes;
use rangemap::RangeInclusiveMap;
use std::{ops::RangeInclusive, vec::Vec};
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
    /// Remap a write memory in a specific address space, clearing previous mappings
    pub fn remap_write_memory(
        &self,
        component_id: ComponentId,
        address_space: AddressSpaceHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();
        let address_space = address_spaces_guard.get_mut(&address_space).unwrap();

        address_space.remap_write_memory(component_id, mapping);
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
        let buffer_subrange = 0..=(buffer.len() - 1);
        let mut remap_callbacks = Vec::default();

        let mut did_handle = false;

        self.write_helper(
            address,
            address_space,
            buffer,
            &mut did_handle,
            buffer_subrange.clone(),
            &mut remap_callbacks,
        )?;

        if !did_handle {
            return Err(WriteMemoryOperationError(RangeInclusiveMap::from_iter([(
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                WriteMemoryOperationErrorFailureType::OutOfBus,
            )])));
        }

        self.process_remap_callbacks(remap_callbacks);

        Ok(())
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    fn write_helper(
        &self,
        address: Address,
        address_space: AddressSpaceHandle,
        buffer: &[u8],
        did_handle: &mut bool,
        buffer_subrange: RangeInclusive<Address>,
        remap_callbacks: &mut Vec<RemapCallback>,
    ) -> Result<(), WriteMemoryOperationError> {
        let address_spaces_guard = self.address_spaces.read().unwrap();
        let address_space_info = address_spaces_guard.get(&address_space).ok_or_else(|| {
            WriteMemoryOperationError(RangeInclusiveMap::from_iter([(
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address),
                WriteMemoryOperationErrorFailureType::OutOfBus,
            )]))
        })?;

        // TODO: Handle width mask wraparound properly
        
        // Crop the accessing range and the address
        let accessing_range = (buffer_subrange.start() + address) & address_space_info.width_mask
            ..=(buffer_subrange.end() + address) & address_space_info.width_mask;
        let address = address & address_space_info.width_mask;

        // Step through the memory translation table
        for (component_assigned_range, component_id) in address_space_info
            .read_members
            .overlapping(accessing_range.clone())
        {
            self.component_store
                .interact_dyn(*component_id, |component| {
                    let adjusted_accessing_range = (*accessing_range
                        .start()
                        .max(component_assigned_range.start()))
                        ..=(*accessing_range.end().min(component_assigned_range.end()));

                    let adjusted_buffer_subrange = (adjusted_accessing_range.start() - address)
                        ..=(adjusted_accessing_range.end() - address);

                    *did_handle = true;

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

                                    self.write_helper(
                                        redirect_address,
                                        redirect_address_space,
                                        buffer,
                                        did_handle,
                                        (range.start() - address)..=(range.end() - address),
                                        remap_callbacks,
                                    )?;
                                }
                            }
                        }

                        remap_callbacks.extend(errors.remap_callback);

                        if !detected_errors.is_empty() {
                            return Err(WriteMemoryOperationError(detected_errors));
                        }
                    }
                    Ok(())
                })
                .unwrap()?;
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
