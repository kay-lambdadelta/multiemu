use crate::{
    component::ErasedComponentHandle,
    memory::{
        Address,
        table::address_space::{MemoryMappingTable, PAGE_SIZE},
    },
};
use multiemu_range::RangeIntersection;
use std::ops::RangeInclusive;

impl MemoryMappingTable {
    #[inline]
    pub fn visit_overlapping<E>(
        &self,
        access_range: RangeInclusive<Address>,
        mut visitor: impl FnMut(
            RangeInclusive<Address>,
            Option<Address>,
            &ErasedComponentHandle,
        ) -> Result<(), E>,
    ) -> Result<(), E> {
        let start = *access_range.start();
        let end = *access_range.end();

        let start_page = start / PAGE_SIZE;
        let end_page = end / PAGE_SIZE;

        for page_index in start_page..=end_page {
            let page = &self.table[page_index];

            let index = page
                .binary_search_by(|entry| {
                    if entry.end < start {
                        std::cmp::Ordering::Less
                    } else if entry.start > start {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Equal
                    }
                })
                .unwrap_or_else(|i| i);

            let left = page[..index]
                .iter()
                .rev()
                .take_while(move |entry| entry.end >= start);

            let right = page[index..]
                .iter()
                .take_while(move |entry| entry.start <= end);

            for (assigned_range, mirror_range, component) in left.chain(right).map(|entry| {
                (
                    entry.start..=entry.end,
                    entry.mirror_start,
                    &entry.component,
                )
            }) {
                visitor(assigned_range.clone(), mirror_range, component)?;

                // If this range completely contains our accessing range we can exit early without more searching
                let cropped_access_range = assigned_range.intersection(&access_range);

                if cropped_access_range == access_range {
                    return Ok(());
                }
            }
        }

        Ok(())
    }
}
