use std::ops::RangeInclusive;

use itertools::Itertools;
use multiemu_range::{ContiguousRange, RangeDifference};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::memory::{MappingEntry, MemoryMappingTable, PAGE_SIZE, TableEntry};

// This function flattens and splits the memory map for faster lookups

impl MemoryMappingTable {
    pub(super) fn commit(&mut self) {
        // Process all pages in parallel
        self.table
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
                            let component = self.registry.get_erased(path).unwrap();

                            vec![TableEntry {
                                start: *source_range.start(),
                                end: *source_range.end(),
                                mirror_start: None,
                                component,
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
                                    let mut source_range = source_range.clone();
                                    let MappingEntry::Component(path) = dest_entry else {
                                        panic!("Recursive mirrors are not allowed");
                                    };
                                    let component = self.registry.get_erased(path).unwrap();

                                    let range_diff =
                                        assigned_destination_range.difference(destination_range);

                                    for exterior_range in range_diff {
                                        if exterior_range.end() < assigned_destination_range.start()
                                        {
                                            let new_start =
                                                source_range.start() + exterior_range.len();

                                            source_range = new_start..=*source_range.end();
                                        }

                                        if assigned_destination_range.start()
                                            < assigned_destination_range.end()
                                        {
                                            let new_end = source_range.end() - exterior_range.len();

                                            source_range = *source_range.start()..=new_end;
                                        }
                                    }

                                    TableEntry {
                                        start: *source_range.start(),
                                        end: *source_range.end(),
                                        mirror_start: Some(*destination_range.start()),
                                        component,
                                    }
                                })
                                .collect()
                        }
                    })
                    .sorted_by_key(|entry| entry.start)
                    .collect();
            });
    }
}
