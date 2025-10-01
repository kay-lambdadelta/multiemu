use crate::{component::ComponentId, machine::registry::ComponentRegistry, memory::Address};
use bitvec::{field::BitField, order::Lsb0};
use nohash::IsEnabled;
use rangemap::RangeInclusiveMap;
use rangetools::Rangetools;
use std::{
    hash::{Hash, Hasher},
    ops::{DerefMut, RangeInclusive},
    sync::{
        Arc, Mutex, RwLock, RwLockWriteGuard,
        atomic::{AtomicBool, Ordering},
    },
    vec::Vec,
};

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct AddressSpaceId(u16);

impl AddressSpaceId {
    pub fn new(id: u16) -> Self {
        Self(id)
    }
}

impl Hash for AddressSpaceId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u16(self.0);
    }
}

impl IsEnabled for AddressSpaceId {}

#[derive(Debug)]
enum MemberEntry {
    Empty,
    Uniform(ComponentId),
    Split(RangeInclusiveMap<Address, ComponentId>),
}

#[derive(Default, Debug)]
pub struct Members {
    read_reference_table: RangeInclusiveMap<Address, ComponentId>,
    write_reference_table: RangeInclusiveMap<Address, ComponentId>,
    read_sharded_table: Vec<MemberEntry>,
    write_sharded_table: Vec<MemberEntry>,
}

#[derive(Debug)]
pub(super) struct AddressSpace {
    pub width_mask: Address,
    width: Address,
    shard_size: Address,
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

        let width = 2usize.pow(address_space_width as u32);

        let shard_size = width / (address_space_width as usize * 2);

        tracing::info!(
            "Address space ({} bits) has a shard size of {}",
            address_space_width,
            shard_size
        );

        Self {
            width_mask,
            width,
            shard_size,
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
        callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.swap(false, Ordering::Acquire) {
            let mut queue_guard = self.queue.lock().unwrap();
            self.update_members(None, queue_guard.drain(..));
        }
        let members = self.members.read().unwrap();
        self.visit_components(accessing_range, callback, &members.read_sharded_table)
    }

    #[inline]
    pub fn visit_write_components<E>(
        &self,
        accessing_range: RangeInclusive<Address>,
        callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
    ) -> Result<(), E> {
        if self.queue_modified.swap(false, Ordering::Acquire) {
            let mut queue_guard = self.queue.lock().unwrap();
            self.update_members(None, queue_guard.drain(..));
        }
        let members = self.members.read().unwrap();
        self.visit_components(accessing_range, callback, &members.write_sharded_table)
    }

    #[inline]
    fn visit_components<E>(
        &self,
        access_range: RangeInclusive<Address>,
        mut callback: impl FnMut(ComponentId, &RangeInclusive<Address>) -> Result<(), E>,
        table: &[MemberEntry],
    ) -> Result<(), E> {
        let first_shard = access_range.start() / self.shard_size;
        let last_shard = access_range.end() / self.shard_size;

        // FIXME: This splits accesses among shards, which is deeply incorrect behavior and only will work right on
        // non io memory and 8 bit machines.

        for shard_id in first_shard..=last_shard {
            let shard_range =
                (shard_id * self.shard_size)..=(shard_id * self.shard_size) + (self.shard_size - 1);

            let adjusted_access_range: RangeInclusive<_> = access_range
                .clone()
                .intersection(shard_range.clone())
                .into();

            if adjusted_access_range.is_empty() {
                continue;
            }

            if let Some(shard) = table.get(shard_id) {
                match shard {
                    MemberEntry::Uniform(component_id) => {
                        callback(*component_id, &adjusted_access_range)?;
                    }
                    MemberEntry::Split(map) => {
                        for (assigned_component_range, component_id) in
                            map.overlapping(adjusted_access_range.clone())
                        {
                            let adjusted_assigned_component_range: RangeInclusive<_> =
                                assigned_component_range
                                    .clone()
                                    .intersection(adjusted_access_range.clone())
                                    .into();

                            callback(*component_id, &adjusted_assigned_component_range)?;
                        }
                    }
                    MemberEntry::Empty => {}
                }
            }
        }

        Ok(())
    }

    #[cold]
    fn update_members(
        &self,
        members: Option<RwLockWriteGuard<'_, Members>>,
        commands: impl IntoIterator<Item = MemoryRemappingCommands>,
    ) {
        let invalid_ranges = (0..self.width).complement();

        let mut members = members.unwrap_or_else(|| self.members.write().unwrap());
        let members = members.deref_mut();

        for command in commands {
            match command {
                MemoryRemappingCommands::Remove { range, types } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} is invalid for a address space that ends at {:04x?}",
                            range,
                            self.width - 1
                        );
                    }

                    tracing::debug!(
                        "Removing memory range {:#04x?} from address space on {:?}",
                        range,
                        types
                    );

                    if types.contains(&MemoryType::Read) {
                        members.read_reference_table.remove(range.clone());
                    }

                    if types.contains(&MemoryType::Write) {
                        members.write_reference_table.remove(range.clone());
                    }
                }
                MemoryRemappingCommands::AddComponent {
                    range,
                    component_id,
                    types,
                } => {
                    if invalid_ranges.clone().intersects(range.clone()) {
                        panic!(
                            "Range {:#04x?} (inserted by component {}) is invalid for a address space that ends at {:04x?} on {:?}",
                            range,
                            self.registry.get_path(component_id),
                            self.width - 1,
                            types
                        );
                    }

                    tracing::debug!(
                        "Mapping memory component {} to address range {:#04x?}",
                        self.registry.get_path(component_id),
                        range
                    );

                    if types.contains(&MemoryType::Read) {
                        members
                            .read_reference_table
                            .insert(range.clone(), component_id);
                    }

                    if types.contains(&MemoryType::Write) {
                        members
                            .write_reference_table
                            .insert(range.clone(), component_id);
                    }
                }
            }
        }

        members.read_sharded_table.clear();
        self.populate_sharded_table(
            &mut members.read_reference_table,
            &mut members.read_sharded_table,
        );

        members.write_sharded_table.clear();
        self.populate_sharded_table(
            &mut members.write_reference_table,
            &mut members.write_sharded_table,
        );
    }

    fn populate_sharded_table(
        &self,
        reference_table: &mut RangeInclusiveMap<Address, ComponentId>,
        sharded_table: &mut Vec<MemberEntry>,
    ) {
        for base_address in (0..self.width).step_by(self.shard_size) {
            let shard_range = base_address..=base_address + (self.shard_size - 1);
            let shard_index = base_address / self.shard_size;
            sharded_table.push(MemberEntry::Empty);

            for (component_assigned_range, component_id) in
                reference_table.overlapping(shard_range.clone())
            {
                let adjusted_component_assigned_range: RangeInclusive<_> = component_assigned_range
                    .clone()
                    .intersection(shard_range.clone())
                    .into();

                let shard = &mut sharded_table[shard_index];

                match shard {
                    MemberEntry::Empty => {
                        *shard = if shard_range == adjusted_component_assigned_range {
                            MemberEntry::Uniform(*component_id)
                        } else {
                            MemberEntry::Split(
                                std::iter::once((
                                    adjusted_component_assigned_range.clone(),
                                    *component_id,
                                ))
                                .collect(),
                            )
                        }
                    }
                    MemberEntry::Uniform(old_component_id) => {
                        let mut map: RangeInclusiveMap<_, _> =
                            std::iter::once((shard_range.clone(), *old_component_id)).collect();
                        map.insert(adjusted_component_assigned_range.clone(), *component_id);

                        *shard = MemberEntry::Split(map);
                    }
                    MemberEntry::Split(map) => {
                        map.insert(adjusted_component_assigned_range.clone(), *component_id);
                    }
                }

                // Component ends here and we can seal up this range
                if adjusted_component_assigned_range.end() == shard_range.end() {
                    match shard {
                        MemberEntry::Split(map) => match map.len() {
                            0 => {
                                *shard = MemberEntry::Empty;
                            }
                            1 => {
                                let gapless = map.gaps(&shard_range).next().is_none();

                                if gapless {
                                    let component_id = *map.get(shard_range.start()).unwrap();

                                    *shard = MemberEntry::Uniform(component_id);
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }
    }
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
