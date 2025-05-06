use super::{
    Address, AddressSpaceHandle,
    callbacks::{ReadMemory, WriteMemory},
};
use crate::memory::{MAX_MEMORY_ACCESS_SIZE, VALID_MEMORY_ACCESS_SIZES};
use arrayvec::ArrayVec;
use bitvec::{field::BitField, order::Lsb0};
use fxhash::FxBuildHasher;
use num::traits::{FromBytes, ToBytes};
use rangemap::RangeInclusiveMap;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BinaryHeap, HashMap},
    fmt::Debug,
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};
use thiserror::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct MemoryHandle(u16);

impl MemoryHandle {
    pub(crate) const fn new(id: u16) -> Self {
        Self(id)
    }
}

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
pub enum ReadMemoryRecord {
    /// Memory could not be read
    Denied,
    /// Memory redirects somewhere else
    Redirect {
        address: Address,
        address_space: AddressSpaceHandle,
    },
}

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

#[derive(Debug)]
struct AddressSpaceInfo {
    width_mask: Address,
    #[allow(unused)]
    name: &'static str,
    read_members: RangeInclusiveMap<Address, MemoryHandle>,
    write_members: RangeInclusiveMap<Address, MemoryHandle>,
}

#[derive(Debug, Default)]
pub struct MemoryTranslationTableImpl {
    current_address_space_id: u16,
    address_spaces: HashMap<AddressSpaceHandle, AddressSpaceInfo, FxBuildHasher>,
    free_address_space_handles: BinaryHeap<AddressSpaceHandle>,

    current_memory_id: u16,
    free_memory_handles: BinaryHeap<MemoryHandle>,

    read_memories: BTreeMap<MemoryHandle, Arc<dyn ReadMemory>>,
    write_memories: BTreeMap<MemoryHandle, Arc<dyn WriteMemory>>,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryTranslationTable(Arc<RwLock<MemoryTranslationTableImpl>>);

impl MemoryTranslationTable {
    pub fn insert_address_space(&self, name: &'static str, width: u8) -> AddressSpaceHandle {
        let mut impl_guard = self.0.write().unwrap();

        let id = if let Some(id) = impl_guard.free_address_space_handles.pop() {
            id
        } else {
            let id = impl_guard.current_address_space_id;
            impl_guard.current_address_space_id = impl_guard
                .current_address_space_id
                .checked_add(1)
                .expect("Too many address spaces");

            AddressSpaceHandle::new(id)
        };

        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..width as usize].fill(true);
        let width_mask = mask.load();

        impl_guard.address_spaces.insert(
            id,
            AddressSpaceInfo {
                width_mask,
                name,
                read_members: RangeInclusiveMap::default(),
                write_members: RangeInclusiveMap::default(),
            },
        );

        id
    }

    pub fn remove_address_space(&self, address_space: AddressSpaceHandle) {
        let mut impl_guard = self.0.write().unwrap();

        impl_guard.address_spaces.remove(&address_space);
        impl_guard.free_address_space_handles.push(address_space);
    }

    pub fn insert_read_memory(
        &self,
        memory: Arc<dyn ReadMemory>,
        mappings: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) {
        let mut impl_guard = self.0.write().unwrap();

        let id = if let Some(id) = impl_guard.free_memory_handles.pop() {
            id
        } else {
            let id = impl_guard.current_memory_id;
            impl_guard.current_memory_id = impl_guard
                .current_memory_id
                .checked_add(1)
                .expect("Too many memories");

            MemoryHandle::new(id)
        };

        impl_guard.read_memories.insert(id, memory);

        for (address_space, addresses) in mappings {
            let address_spaces = impl_guard
                .address_spaces
                .get_mut(&address_space)
                .expect("Non existant address space");

            assert!(
                !address_spaces.read_members.overlaps(&addresses),
                "Addresses {:x?} conflict with already existing bus infrastructure {:x?}",
                addresses,
                address_spaces
            );

            address_spaces.read_members.insert(addresses, id);
        }
    }

    pub fn insert_write_memory(
        &self,
        memory: Arc<dyn WriteMemory>,
        mappings: impl IntoIterator<Item = (AddressSpaceHandle, RangeInclusive<usize>)>,
    ) {
        let mut impl_guard = self.0.write().unwrap();

        let id = if let Some(id) = impl_guard.free_memory_handles.pop() {
            id
        } else {
            let id = impl_guard.current_memory_id;
            impl_guard.current_memory_id = impl_guard
                .current_memory_id
                .checked_add(1)
                .expect("Too many memories");

            MemoryHandle::new(id)
        };

        impl_guard.write_memories.insert(id, memory);

        for (address_space, addresses) in mappings {
            let address_spaces = impl_guard
                .address_spaces
                .get_mut(&address_space)
                .expect("Non existant address space");

            assert!(
                !address_spaces.write_members.overlaps(&addresses),
                "Addresses {:x?} conflict with already existing bus infrastructure {:x?}",
                addresses,
                address_spaces
            );

            address_spaces.write_members.insert(addresses, id);
        }
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
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );
        let impl_guard = self.0.read().unwrap();

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            address_space,
            (0..=buffer.len() - 1),
        )]);

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
                    .read_memories
                    .get(memory_id)
                    .expect("Non existant memory");

                did_handle = true;
                let mut errors = RangeInclusiveMap::default();

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                memory.read_memory(
                    *overlap_start,
                    address_space,
                    &mut buffer[buffer_subrange.clone()],
                    &mut errors,
                );

                let mut detected_errors = RangeInclusiveMap::default();

                for (range, error) in errors {
                    match error {
                        ReadMemoryRecord::Denied => {
                            tracing::error!(
                                "Read memory operation operation denied at {:#04x?}",
                                range
                            );

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
                                "Component attempted to redirect to itself {:x?} -> {:x}",
                                component_assignment_range,
                                redirect_address,
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
                    return Err(ReadMemoryOperationError(detected_errors));
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

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            address_space,
            (0..=(buffer.len() - 1)),
        )]);

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
                    .write_memories
                    .get(memory_id)
                    .expect("Non existant memory");

                did_handle = true;
                let mut errors = RangeInclusiveMap::default();

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                memory.write_memory(
                    *overlap_start,
                    address_space,
                    &buffer[buffer_subrange.clone()],
                    &mut errors,
                );

                let mut detected_errors = RangeInclusiveMap::default();

                for (range, error) in errors {
                    match error {
                        WriteMemoryRecord::Denied => {
                            tracing::error!(
                                "Write memory operation operation denied at {:#04x?}",
                                range
                            );

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
                                "Component attempted to redirect to itself {:x?} -> {:x}",
                                component_assignment_range,
                                redirect_address,
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

    #[inline]
    pub fn preview(
        &self,
        address: usize,
        address_space: AddressSpaceHandle,
        buffer: &mut [u8],
    ) -> Result<(), PreviewMemoryOperationError> {
        let impl_guard = self.0.write().unwrap();

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            address_space,
            (0..=(buffer.len() - 1)),
        )]);

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
                    .read_memories
                    .get(memory_id)
                    .expect("Non existant memory");

                did_handle = true;
                let mut errors = RangeInclusiveMap::default();

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                memory.preview_memory(
                    *overlap_start,
                    address_space,
                    &mut buffer[buffer_subrange.clone()],
                    &mut errors,
                );

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
                                "Component attempted to redirect to itself {:x?} -> {:x}",
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
                            detected_errors
                                .insert(range, PreviewMemoryOperationErrorFailureType::Impossible);
                        }
                    }
                }

                if !detected_errors.is_empty() {
                    return Err(PreviewMemoryOperationError(detected_errors));
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
