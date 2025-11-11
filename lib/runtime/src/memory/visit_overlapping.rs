use crate::{
    component::ErasedComponentHandle,
    memory::{Address, MemoryMappingTable, PAGE_SIZE},
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

            for entry in page
                .iter()
                .filter(|entry| access_range.intersects(&(entry.start..=entry.end)))
            {
                let entry_range = entry.start..=entry.end;

                visitor(entry_range.clone(), entry.mirror_start, &entry.component)?;

                // If this range completely contains our accessing range we can exit early without more searching
                let cropped_access_range = entry_range.intersection(&access_range);

                if cropped_access_range == access_range {
                    return Ok(());
                }
            }
        }

        Ok(())
    }
}
