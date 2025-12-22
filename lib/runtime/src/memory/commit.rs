use std::ops::RangeInclusive;

use bytes::Bytes;
use fluxemu_range::{ContiguousRange, RangeIntersection};
use itertools::Itertools;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::{
    machine::registry::ComponentRegistry,
    memory::{
        Address, ComputedTablePage, ComputedTablePageTarget, MappingEntry, MemoryMappingTable,
        PAGE_SIZE,
    },
    path::FluxEmuPath,
};

#[derive(Debug, Clone)]
pub enum MapTarget {
    Component(FluxEmuPath),
    Memory(FluxEmuPath),
    Mirror {
        destination: RangeInclusive<Address>,
    },
}

/// Command for how the memory access table should modify the memory map
#[allow(missing_docs)]
#[derive(Debug, Clone)]
pub enum MemoryRemappingCommand {
    /// Add a target to the memory map, or add a map to an existing one
    Map {
        range: RangeInclusive<Address>,
        target: MapTarget,
        permissions: Permissions,
    },
    /// Clear a memory range
    Unmap {
        range: RangeInclusive<Address>,
        permissions: Permissions,
    },
    /// Register a buffer or another item
    Register { path: FluxEmuPath, buffer: Bytes },
}

#[allow(missing_docs)]
#[derive(Debug, Clone, Copy)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
}

impl Permissions {
    /// Instance of [Self] where everything is allowed
    pub fn all() -> Self {
        Self {
            read: true,
            write: true,
        }
    }
}

// This function flattens and splits the memory map for faster lookups

impl MemoryMappingTable {
    pub(super) fn commit(
        &mut self,
        registry: &ComponentRegistry,
        resources: &scc::HashMap<FluxEmuPath, Bytes>,
    ) {
        // Process all pages in parallel
        self.computed_table
            .par_iter_mut()
            .enumerate()
            .for_each(|(page_index, page)| {
                let base = page_index * PAGE_SIZE;
                let end = base + PAGE_SIZE - 1;
                let page_range = base..=end;

                *page = self
                    .master
                    .overlapping(page_range.clone())
                    .map(|(range, component)| (range.clone(), component))
                    .flat_map(|(source_range, entry)| match entry {
                        MappingEntry::Component(path) => {
                            let component = registry.handle(path).unwrap();

                            vec![ComputedTablePage {
                                range: source_range,
                                target: ComputedTablePageTarget::Component {
                                    mirror_start: None,
                                    component,
                                },
                            }]
                        }
                        MappingEntry::Mirror {
                            source_base,
                            destination_base,
                        } => {
                            let offset = source_range
                                .start()
                                .checked_sub(*source_base)
                                .expect("mirror source_range.start must be >= source_base");

                            let source_length = source_range.len();

                            let destination_start = destination_base + offset;
                            let assigned_destination_range = RangeInclusive::from_start_and_length(
                                destination_start,
                                source_length,
                            );

                            self.master
                                .overlapping(assigned_destination_range.clone())
                                .map(|(destination_range, dest_entry)| {
                                    let dest_overlap =
                                        assigned_destination_range.intersection(destination_range);

                                    let shrink_left =
                                        dest_overlap.start() - assigned_destination_range.start();
                                    let shrink_right =
                                        assigned_destination_range.end() - dest_overlap.end();

                                    let calculated_source_range = (source_range.start()
                                        + shrink_left)
                                        ..=(source_range.end() - shrink_right);

                                    match dest_entry {
                                        MappingEntry::Component(component_path) => {
                                            let component =
                                                registry.handle(component_path).unwrap();

                                            ComputedTablePage {
                                                range: calculated_source_range,
                                                target: ComputedTablePageTarget::Component {
                                                    mirror_start: Some(*destination_range.start()),
                                                    component,
                                                },
                                            }
                                        }
                                        MappingEntry::Mirror { .. } => {
                                            panic!("Recursive mirrors are not allowed");
                                        }
                                        MappingEntry::Memory(resource_path) => {
                                            let memory = resources.get_sync(resource_path).unwrap();
                                            let destination_overlap = assigned_destination_range
                                                .intersection(destination_range);

                                            assert_eq!(
                                                destination_overlap.len(),
                                                calculated_source_range.len()
                                            );

                                            let buffer_subrange = (destination_overlap.start()
                                                - destination_range.start())
                                                ..=(destination_overlap.end()
                                                    - destination_range.start());

                                            let memory = memory.slice(buffer_subrange);

                                            ComputedTablePage {
                                                range: calculated_source_range,
                                                target: ComputedTablePageTarget::Memory(memory),
                                            }
                                        }
                                    }
                                })
                                .collect()
                        }
                        MappingEntry::Memory(resource_path) => {
                            let memory = resources.get_sync(resource_path).unwrap();

                            assert_eq!(memory.len(), source_range.len());

                            vec![ComputedTablePage {
                                range: source_range,
                                target: ComputedTablePageTarget::Memory(memory.clone()),
                            }]
                        }
                    })
                    .sorted_by_key(|entry| *entry.range.start())
                    .collect();
            });
    }
}
