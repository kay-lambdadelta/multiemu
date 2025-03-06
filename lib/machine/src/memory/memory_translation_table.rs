use super::AddressSpaceId;
use super::callbacks::{PreviewMemory, ReadMemory, WriteMemory};
use crate::memory::{MAX_MEMORY_ACCESS_SIZE, VALID_MEMORY_ACCESS_SIZES};
use arrayvec::ArrayVec;
use bitvec::{field::BitField, order::Lsb0, view::BitView};
use fxhash::FxBuildHasher;
use rangemap::RangeInclusiveMap;
use std::fmt::{Debug, Formatter};
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::{collections::HashMap, sync::atomic::AtomicBool};
use thiserror::Error;

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
    read: RangeInclusiveMap<usize, MemoryOperatorWrapper<dyn ReadMemory>>,
    write: RangeInclusiveMap<usize, MemoryOperatorWrapper<dyn WriteMemory>>,
    preview: RangeInclusiveMap<usize, MemoryOperatorWrapper<dyn PreviewMemory>>,
    width: u8,
}

struct MemoryOperatorWrapper<T: ?Sized + Sync>(pub Arc<T>);

impl<T: ?Sized + Sync> Clone for MemoryOperatorWrapper<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: ?Sized + Sync> PartialEq for MemoryOperatorWrapper<T> {
    fn eq(&self, _other: &Self) -> bool {
        // HACK: Implements equality so rangemap operates correctly
        //
        // Remove this when rangemap allows losing the eq requirement
        false
    }
}

impl<T: ?Sized + Sync> Eq for MemoryOperatorWrapper<T> {}

impl<T: ?Sized + Sync> Debug for MemoryOperatorWrapper<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("MemoryOperatorWrapper").finish()
    }
}

#[derive(Default, Debug)]
pub struct MemoryTranslationTable {
    bus_info: HashMap<AddressSpaceId, BusInfo, FxBuildHasher>,
    dirty_pages: Vec<AtomicBool>,
}

impl MemoryTranslationTable {
    pub fn insert_bus(&mut self, address_space_id: AddressSpaceId, bus_width: u8) {
        self.bus_info.insert(
            address_space_id,
            BusInfo {
                width: bus_width,
                read: RangeInclusiveMap::default(),
                write: RangeInclusiveMap::default(),
                preview: RangeInclusiveMap::default(),
            },
        );
    }

    pub fn insert_read_callback(
        &mut self,
        address_space: AddressSpaceId,
        mappings: impl IntoIterator<Item = RangeInclusive<usize>>,
        read_callback: Arc<dyn ReadMemory>,
    ) {
        self.bus_info
            .get_mut(&address_space)
            .expect("Non existant address space")
            .read
            .extend(
                mappings
                    .into_iter()
                    .map(|range| (range, MemoryOperatorWrapper(read_callback.clone()))),
            );
    }

    pub fn insert_write_callback(
        &mut self,
        address_space: AddressSpaceId,
        mappings: impl IntoIterator<Item = RangeInclusive<usize>>,
        write_callback: Arc<dyn WriteMemory>,
    ) {
        self.bus_info
            .get_mut(&address_space)
            .expect("Non existant address space")
            .write
            .extend(
                mappings
                    .into_iter()
                    .map(|range| (range, MemoryOperatorWrapper(write_callback.clone()))),
            );
    }

    pub fn insert_preview_callback(
        &mut self,
        address_space: AddressSpaceId,
        mappings: impl IntoIterator<Item = RangeInclusive<usize>>,
        preview_callback: Arc<dyn PreviewMemory>,
    ) {
        self.bus_info
            .get_mut(&address_space)
            .expect("Non existant address space")
            .preview
            .extend(
                mappings
                    .into_iter()
                    .map(|range| (range, MemoryOperatorWrapper(preview_callback.clone()))),
            );
    }

    pub fn address_spaces(&self) -> u8 {
        self.bus_info
            .len()
            .try_into()
            .expect("Too many address spaces!")
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

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            (0..=buffer.len() - 1),
        )]);

        while let Some((address, buffer_subrange)) = needed_accesses.pop() {
            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, read_callback) in
                bus_info.read.overlapping(accessing_range.clone())
            {
                did_handle = true;
                let mut errors = RangeInclusiveMap::default();

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                read_callback.0.read_memory(
                    *overlap_start,
                    &mut buffer[buffer_subrange.clone()],
                    address_space,
                    &mut errors,
                );

                let mut detected_errors = RangeInclusiveMap::default();

                for (range, error) in errors {
                    match error {
                        ReadMemoryRecord::Denied => {
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

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            (0..=(buffer.len() - 1)),
        )]);

        while let Some((address, buffer_subrange)) = needed_accesses.pop() {
            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, write_callback) in
                bus_info.write.overlapping(accessing_range.clone())
            {
                did_handle = true;
                let mut errors = RangeInclusiveMap::default();

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                write_callback.0.write_memory(
                    *overlap_start,
                    &buffer[buffer_subrange.clone()],
                    address_space,
                    &mut errors,
                );

                let mut detected_errors = RangeInclusiveMap::default();

                for (range, error) in errors {
                    match error {
                        WriteMemoryRecord::Denied => {
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

        let mut needed_accesses = ArrayVec::<_, { MAX_MEMORY_ACCESS_SIZE }>::from_iter([(
            address,
            (0..=(buffer.len() - 1)),
        )]);

        while let Some((address, buffer_subrange)) = needed_accesses.pop() {
            let accessing_range =
                (buffer_subrange.start() + address)..=(buffer_subrange.end() + address);
            let mut did_handle = false;

            for (component_assignment_range, preview_callback) in
                bus_info.preview.overlapping(accessing_range.clone())
            {
                did_handle = true;
                let mut errors = RangeInclusiveMap::default();

                let overlap_start = accessing_range
                    .start()
                    .max(component_assignment_range.start());

                preview_callback.0.preview_memory(
                    *overlap_start,
                    &mut buffer[buffer_subrange.clone()],
                    address_space,
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
}
