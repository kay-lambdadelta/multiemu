use super::{AddressSpaceId, callbacks::Memory};
use crate::memory::{MAX_MEMORY_ACCESS_SIZE, VALID_MEMORY_ACCESS_SIZES};
use arrayvec::ArrayVec;
use bitvec::{field::BitField, order::Lsb0, view::BitView};
use fxhash::FxBuildHasher;
use num::traits::{FromBytes, ToBytes};
use rangemap::RangeInclusiveMap;
use std::{collections::HashMap, fmt::Debug, ops::RangeInclusive};
use thiserror::Error;

type MemoryId = usize;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ReadMemoryOperationErrorFailureType {
    Denied,
    OutOfBus,
}

#[derive(Error, Debug)]
#[error("Read operation failed: {0:#?}")]
pub struct ReadMemoryOperationError(RangeInclusiveMap<usize, ReadMemoryOperationErrorFailureType>);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WriteMemoryOperationErrorFailureType {
    Denied,
    OutOfBus,
}

#[derive(Error, Debug)]
#[error("Write operation failed: {0:#?}")]
pub struct WriteMemoryOperationError(
    RangeInclusiveMap<usize, WriteMemoryOperationErrorFailureType>,
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
    RangeInclusiveMap<usize, PreviewMemoryOperationErrorFailureType>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadMemoryRecord {
    /// Memory could not be read
    Denied,
    /// Memory redirects somewhere else
    Redirect { address: usize },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WriteMemoryRecord {
    /// Memory could not be written
    Denied,
    /// Memory redirects somewhere else
    Redirect { address: usize },
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PreviewMemoryRecord {
    /// Memory denied
    Denied,
    /// Memory redirects somewhere else
    Redirect {
        address: usize,
    },
    // Memory here can't be read without an intense calculation or a state change
    Impossible,
}

#[derive(Debug)]
struct BusInfo {
    members: RangeInclusiveMap<usize, MemoryId>,
    width: u8,
}

#[derive(Default, Debug)]
pub struct MemoryTranslationTable {
    current_memory_id: MemoryId,
    bus_info: HashMap<AddressSpaceId, BusInfo, FxBuildHasher>,
    memories: Vec<Box<dyn Memory>>,
}

impl MemoryTranslationTable {
    pub fn insert_address_space(&mut self, address_space_id: AddressSpaceId, bus_width: u8) {
        self.bus_info.insert(
            address_space_id,
            BusInfo {
                width: bus_width,
                members: RangeInclusiveMap::new(),
            },
        );
    }

    pub fn insert_memory(
        &mut self,
        mappings: impl IntoIterator<Item = (RangeInclusive<usize>, AddressSpaceId)>,
        memory: Box<dyn Memory>,
    ) {
        let id = self.current_memory_id;
        self.current_memory_id = self
            .current_memory_id
            .checked_add(1)
            .expect("Too many memories");

        self.memories.insert(id, memory);

        for (addresses, address_space) in mappings {
            self.bus_info
                .get_mut(&address_space)
                .expect("Non existant address space")
                .members
                .insert(addresses, id);
        }
    }

    pub fn address_spaces(&self) -> impl IntoIterator<Item = AddressSpaceId> + '_ {
        self.bus_info.keys().copied()
    }

    pub fn get_address_space_width(&self, address_space: AddressSpaceId) -> Option<u8> {
        self.bus_info.get(&address_space).map(|bus| bus.width)
    }

    /// Step through the memory translation table to fill the buffer with data
    ///
    /// Contents of the buffer upon failure are usually component specific
    #[inline]
    pub fn read(
        &self,
        address: usize,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), ReadMemoryOperationError> {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        let bus_info = self
            .bus_info
            .get(&address_space)
            .expect("Non existant address space");

        // Cut off address
        let address = address.view_bits::<Lsb0>()[..bus_info.width as usize].load_le::<usize>();

        tracing::trace!(
            "Reading memory at {:#04x?} with size {}",
            address,
            buffer.len()
        );

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            (0..=buffer.len() - 1),
        )]);

        while let Some((address, buffer_subrange)) = needed_accesses.pop() {
            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, memory_id) in
                bus_info.members.overlapping(accessing_range.clone())
            {
                let memory = self.memories.get(*memory_id).expect("Non existant memory");

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
                        } => {
                            assert!(
                                !component_assignment_range.contains(&redirect_address),
                                "Component attempted to redirect to itself"
                            );

                            needed_accesses.push((
                                redirect_address,
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
        address: usize,
        address_space: AddressSpaceId,
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
        address_space: AddressSpaceId,
        buffer: &[u8],
    ) -> Result<(), WriteMemoryOperationError> {
        assert!(
            VALID_MEMORY_ACCESS_SIZES.contains(&buffer.len()),
            "Invalid memory access size {}",
            buffer.len()
        );

        let bus_info = self
            .bus_info
            .get(&address_space)
            .expect("Non existant address space");

        let address = address.view_bits::<Lsb0>()[..bus_info.width as usize].load_le::<usize>();

        tracing::trace!(
            "Writing memory at {:#04x?} with size {}",
            address,
            buffer.len()
        );

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            (0..=(buffer.len() - 1)),
        )]);

        while let Some((address, buffer_subrange)) = needed_accesses.pop() {
            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, memory_id) in
                bus_info.members.overlapping(accessing_range.clone())
            {
                let memory = self.memories.get(*memory_id).expect("Non existant memory");

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
                        } => {
                            assert!(
                                !component_assignment_range.contains(&redirect_address),
                                "Component attempted to redirect to itself"
                            );

                            needed_accesses.push((
                                redirect_address,
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
        address_space: AddressSpaceId,
        value: T,
    ) -> Result<(), WriteMemoryOperationError> {
        self.write(address, address_space, value.to_le_bytes().as_ref())
    }

    #[inline]
    pub fn write_be_value<T: ToBytes>(
        &self,
        address: usize,
        address_space: AddressSpaceId,
        value: T,
    ) -> Result<(), WriteMemoryOperationError> {
        self.write(address, address_space, value.to_be_bytes().as_ref())
    }

    #[inline]
    pub fn preview(
        &self,
        address: usize,
        address_space: AddressSpaceId,
        buffer: &mut [u8],
    ) -> Result<(), PreviewMemoryOperationError> {
        let bus_info = self
            .bus_info
            .get(&address_space)
            .expect("Non existant address space");

        let address = address.view_bits::<Lsb0>()[..bus_info.width as usize].load_le::<usize>();

        tracing::trace!(
            "Previewing memory at {:#04x?} with size {}",
            address,
            buffer.len()
        );

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            (0..=(buffer.len() - 1)),
        )]);

        while let Some((address, buffer_subrange)) = needed_accesses.pop() {
            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, memory_id) in
                bus_info.members.overlapping(accessing_range.clone())
            {
                let memory = self.memories.get(*memory_id).expect("Non existant memory");

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
                        } => {
                            assert!(
                                !component_assignment_range.contains(&redirect_address),
                                "Component attempted to redirect to itself"
                            );

                            needed_accesses.push((
                                redirect_address,
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
        address: usize,
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
