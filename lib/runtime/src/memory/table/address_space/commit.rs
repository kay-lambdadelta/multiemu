use crate::memory::table::address_space::MappingEntry;
use crate::memory::table::address_space::MemoryMappingTable;
use crate::memory::table::address_space::PAGE_SIZE;
use crate::memory::table::address_space::TableEntry;
use itertools::Itertools;
use multiemu_range::ContiguousRange;
use rayon::iter::IndexedParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;
use std::ops::RangeInclusive;

// This function flattens and splits the memory map for faster lookups

impl MemoryMappingTable {
    pub fn commit(&mut self) {
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
                            let destination_range = RangeInclusive::from_start_and_length(
                                destination_start,
                                source_length,
                            );

                            self.master
                                .overlapping(destination_range)
                                .map(|(destination_range, dest_entry)| {
                                    let MappingEntry::Component(path) = dest_entry else {
                                        panic!("Recursive mirrors are not allowed");
                                    };
                                    let component = self.registry.get_erased(path).unwrap();

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
