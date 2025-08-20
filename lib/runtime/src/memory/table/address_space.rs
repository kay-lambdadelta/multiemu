use crate::{
    component::{ComponentId, ComponentRegistry},
    memory::Address,
};
use bitvec::{field::BitField, order::Lsb0};
use nohash::{BuildNoHashHasher, IsEnabled};
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    ops::RangeInclusive,
    sync::{
        Arc, Mutex, RwLock, RwLockWriteGuard,
        atomic::{AtomicBool, Ordering},
    },
    vec::Vec,
};

const PAGE_SIZE: usize = 4096;

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct AddressSpaceHandle(u16);

impl AddressSpaceHandle {
    pub fn new(id: u16) -> Self {
        Self(id)
    }
}

impl Hash for AddressSpaceHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u16(self.0);
    }
}

impl IsEnabled for AddressSpaceHandle {}

#[derive(Default, Debug, Clone)]
pub struct Members {
    read: HashMap<Address, [Option<ComponentId>; PAGE_SIZE], BuildNoHashHasher<Address>>,
    write: HashMap<Address, [Option<ComponentId>; PAGE_SIZE], BuildNoHashHasher<Address>>,
    read_mappings: Vec<Vec<RangeInclusive<Address>>>,
    write_mappings: Vec<Vec<RangeInclusive<Address>>>,
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    max_value: Address,
    members: RwLock<Members>,
    /// Queue for if the address space is locked at the moment
    queue: Mutex<Vec<MemoryRemappingCommands>>,
    queue_modified: AtomicBool,
    registry: Arc<ComponentRegistry>,
}

impl AddressSpace {
    pub fn new(registry: Arc<ComponentRegistry>, address_space_width: u8) -> Self {
        let mut mask = bitvec::bitvec![usize, Lsb0; 0; usize::BITS as usize];
        mask[..address_space_width as usize].fill(true);
        let width_mask = mask.load_le();

        let max_value = 2usize.pow(address_space_width as u32) - 1;

        Self {
            width_mask,
            max_value,
            members: Default::default(),
            queue: Default::default(),
            queue_modified: AtomicBool::new(false),
            registry,
        }
    }

    pub fn remap(&self, commands: impl IntoIterator<Item = MemoryRemappingCommands>) {
        let mut queue_guard = self.queue.lock().unwrap();

        if let Ok(members) = self.members.try_write() {
            self.update_members(Some(members), commands);
        } else {
            queue_guard.extend(commands);
            self.queue_modified.store(true, Ordering::Release);
        }
    }

    #[inline]
    pub fn visit_read_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.swap(false, Ordering::Acquire) {
            let mut queue_guard = self.queue.lock().unwrap();
            self.update_members(None, queue_guard.drain(..));
        }

        let members = self.members.read().unwrap();

        let start_chunk = accessing_range.start() / PAGE_SIZE;
        let end_chunk = accessing_range.end() / PAGE_SIZE;

        for chunk_index in start_chunk..=end_chunk {
            if let Some(chunk) = members.read.get(&(chunk_index * PAGE_SIZE)) {
                let chunk_start = if chunk_index == start_chunk {
                    accessing_range.start() % PAGE_SIZE
                } else {
                    0
                };
                let chunk_end = if chunk_index == end_chunk {
                    accessing_range.end() % PAGE_SIZE
                } else {
                    PAGE_SIZE - 1
                };
                let mut last_component = None;

                for inner_idx in chunk_start..=chunk_end {
                    let current_component = chunk[inner_idx];

                    if current_component != last_component
                        && let Some(component_id) = last_component
                    {
                        read_callback_helper(
                            &accessing_range,
                            &mut callback,
                            &members,
                            component_id,
                        )?;
                    }

                    last_component = current_component;
                }

                if let Some(component_id) = last_component {
                    read_callback_helper(&accessing_range, &mut callback, &members, component_id)?;
                }
            }
        }

        Ok(())
    }

    #[inline]
    pub fn visit_write_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.swap(false, Ordering::Acquire) {
            let mut queue_guard = self.queue.lock().unwrap();
            self.update_members(None, queue_guard.drain(..));
        }

        let members = self.members.read().unwrap();

        let start_chunk = accessing_range.start() / PAGE_SIZE;
        let end_chunk = accessing_range.end() / PAGE_SIZE;

        for chunk_index in start_chunk..=end_chunk {
            if let Some(chunk) = members.write.get(&(chunk_index * PAGE_SIZE)) {
                let chunk_start = if chunk_index == start_chunk {
                    accessing_range.start() % PAGE_SIZE
                } else {
                    0
                };
                let chunk_end = if chunk_index == end_chunk {
                    accessing_range.end() % PAGE_SIZE
                } else {
                    PAGE_SIZE - 1
                };
                let mut last_component = None;

                for inner_idx in chunk_start..=chunk_end {
                    let current_component = chunk[inner_idx];

                    if current_component != last_component
                        && let Some(component_id) = last_component
                    {
                        write_callback_helper(
                            &accessing_range,
                            &mut callback,
                            &members,
                            component_id,
                        )?;
                    }

                    last_component = current_component;
                }

                if let Some(component_id) = last_component {
                    write_callback_helper(&accessing_range, &mut callback, &members, component_id)?;
                }
            }
        }

        Ok(())
    }

    fn update_members(
        &self,
        members: Option<RwLockWriteGuard<'_, Members>>,
        commands: impl IntoIterator<Item = MemoryRemappingCommands>,
    ) {
        let mut read_members: RangeInclusiveMap<_, _, Address> = RangeInclusiveMap::default();
        let mut write_members: RangeInclusiveMap<_, _, Address> = RangeInclusiveMap::default();

        let invalid_ranges = (0..=self.max_value).complement();

        let mut members = members.unwrap_or_else(|| self.members.write().unwrap());

        for (component, range) in
            members
                .read_mappings
                .drain(..)
                .enumerate()
                .flat_map(|(id, ranges)| {
                    ranges
                        .into_iter()
                        // It's ok to use as instead of try into here, someone else checked it when creating the id
                        .map(move |range| (ComponentId::new(id as u16), range))
                })
        {
            read_members.insert(range, component);
        }

        for (component, range) in
            members
                .write_mappings
                .drain(..)
                .enumerate()
                .flat_map(|(id, ranges)| {
                    ranges
                        .into_iter()
                        // It's ok to use as instead of try into here, someone else checked it when creating the id
                        .map(move |range| (ComponentId::new(id as u16), range))
                })
        {
            write_members.insert(range, component);
        }

        for command in commands {
            match command {
                MemoryRemappingCommands::Remove { range, types } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?}",
                            range, self.max_value
                        );
                    }

                    tracing::debug!("Removing memory range {:#04x?} from address space", range);

                    if types.contains(&MemoryType::Read) {
                        read_members.remove(range.clone());
                    }

                    if types.contains(&MemoryType::Write) {
                        write_members.remove(range.clone());
                    }
                }
                MemoryRemappingCommands::AddComponent {
                    range,
                    component_id,
                    types,
                } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} (inserted by component {}) is invalid for a address space that ends at {:04x?}",
                            range,
                            self.registry.get_path(component_id),
                            self.max_value
                        );
                    }

                    tracing::debug!(
                        "Mapping memory component_id {:?} to address range {:#04x?}",
                        component_id,
                        range
                    );

                    if types.contains(&MemoryType::Read) {
                        read_members.insert(range.clone(), component_id);
                    }

                    if types.contains(&MemoryType::Write) {
                        write_members.insert(range.clone(), component_id);
                    }
                }
            }
        }

        members.read.clear();
        members.write.clear();
        members.read_mappings.clear();
        members.write_mappings.clear();

        for (addresses, component_id) in read_members {
            let start_chunk = addresses.start() / PAGE_SIZE;
            let end_chunk = addresses.end() / PAGE_SIZE;

            for chunk_index in start_chunk..=end_chunk {
                let chunk = members
                    .read
                    .entry(chunk_index * PAGE_SIZE)
                    .or_insert_with(|| [None; PAGE_SIZE]);

                let chunk_start = if chunk_index == start_chunk {
                    addresses.start() % PAGE_SIZE
                } else {
                    0
                };

                let chunk_end = if chunk_index == end_chunk {
                    addresses.end() % PAGE_SIZE
                } else {
                    PAGE_SIZE - 1
                };

                let chunk_range = chunk_start..=chunk_end;

                chunk[chunk_range].fill(Some(component_id));
            }

            let old_len = members.read_mappings.len();
            members
                .read_mappings
                .resize_with((component_id.get() as usize + 1).max(old_len), Vec::new);
            members.read_mappings[component_id.get() as usize].push(addresses);
        }

        for (addresses, component_id) in write_members {
            let start_chunk = addresses.start() / PAGE_SIZE;
            let end_chunk = addresses.end() / PAGE_SIZE;

            for chunk_index in start_chunk..=end_chunk {
                let chunk = members
                    .write
                    .entry(chunk_index * PAGE_SIZE)
                    .or_insert_with(|| [None; PAGE_SIZE]);

                let chunk_start = if chunk_index == start_chunk {
                    addresses.start() % PAGE_SIZE
                } else {
                    0
                };

                let chunk_end = if chunk_index == end_chunk {
                    addresses.end() % PAGE_SIZE
                } else {
                    PAGE_SIZE - 1
                };

                let chunk_range = chunk_start..=chunk_end;

                chunk[chunk_range].fill(Some(component_id));
            }

            let old_len = members.write_mappings.len();
            members
                .write_mappings
                .resize_with((component_id.get() as usize + 1).max(old_len), Vec::new);
            members.write_mappings[component_id.get() as usize].push(addresses);
        }
    }
}

#[inline(always)]
fn read_callback_helper<E>(
    accessing_range: &RangeInclusive<usize>,
    callback: &mut impl FnMut(ComponentId, &RangeInclusive<usize>) -> Result<(), E>,
    members: &Members,
    component_id: ComponentId,
) -> Result<(), E> {
    let assigned_ranges = &members.read_mappings[component_id.get() as usize];

    let component_assigned_range = assigned_ranges
        .iter()
        .find(|range| (*range).clone().intersects(accessing_range.clone()))
        .expect("Severe logical error");

    callback(component_id, component_assigned_range)?;

    Ok(())
}

#[inline(always)]
fn write_callback_helper<E>(
    accessing_range: &RangeInclusive<usize>,
    callback: &mut impl FnMut(ComponentId, &RangeInclusive<usize>) -> Result<(), E>,
    members: &Members,
    component_id: ComponentId,
) -> Result<(), E> {
    let assigned_ranges = &members.write_mappings[component_id.get() as usize];

    let component_assigned_range = assigned_ranges
        .iter()
        .find(|range| (*range).clone().intersects(accessing_range.clone()))
        .expect("Severe logical error");

    callback(component_id, component_assigned_range)?;

    Ok(())
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum MemoryType {
    Read,
    Write,
}

#[derive(Debug)]
pub enum MemoryRemappingCommands {
    AddComponent {
        range: RangeInclusive<Address>,
        component_id: ComponentId,
        types: Vec<MemoryType>,
    },
    Remove {
        range: RangeInclusive<Address>,
        types: Vec<MemoryType>,
    },
}
