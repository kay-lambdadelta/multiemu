use super::{
    MemoryTranslationTable, NEEDED_ACCESSES_BASE_CAPACITY, NeededAccess, RemapCallback,
    address_space::{AddressSpace, AddressSpaceHandle},
};
use crate::{
    component::{ComponentId, ComponentStore},
    memory::Address,
};
use num::traits::ToBytes;
use rangemap::RangeInclusiveMap;
use smallvec::SmallVec;
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

impl MemoryTranslationTable {
    /// Remap a write memory in a specific address space, clearing previous mappings
    pub fn remap_write_memory(
        &self,
        component_id: ComponentId,
        address_space: AddressSpaceHandle,
        mapping: impl IntoIterator<Item = RangeInclusive<Address>>,
    ) {
        let mut address_spaces_guard = self.address_spaces.write().unwrap();
        let address_space = address_spaces_guard
            .get_mut(address_space.get() as usize)
            .unwrap();

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
        let mut needed_accesses = SmallVec::from_iter([NeededAccess {
            address,
            address_space,
            buffer_subrange: (0..=(buffer.len() - 1)),
        }]);
        let mut remap_callbacks = Vec::default();
        let component_store = self.component_store.upgrade().unwrap();

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
                let address_space_info = address_spaces_guard
                    .get(address_space.get() as usize)
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
                    &component_store,
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
        component_store: &ComponentStore,
        needed_accesses: &mut SmallVec<NeededAccess, NEEDED_ACCESSES_BASE_CAPACITY>,
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

        for (component_assignment_range, component_id) in address_space_info
            .write_members
            .overlapping(accessing_range.clone())
        {
            let adjusted_accessing_range = (*accessing_range
                .start()
                .max(component_assignment_range.start()))
                ..=(*accessing_range.end().min(component_assignment_range.end()));

            let adjusted_buffer_subrange = (adjusted_accessing_range.start() - address)
                ..=(adjusted_accessing_range.end() - address);

            component_store.interact_dyn(*component_id, |component| {
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
            }).unwrap()?;
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
