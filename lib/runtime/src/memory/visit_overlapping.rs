use std::ops::RangeInclusive;

use multiemu_range::RangeIntersection;

use crate::{
    component::ErasedComponentHandle,
    memory::{Address, MemoryError, MemoryErrorType, MemoryMappingTable, PAGE_SIZE},
};

impl MemoryMappingTable {
    #[inline]
    pub fn visit_overlapping(
        &self,
        access_range: RangeInclusive<Address>,
        mut visitor: impl FnMut(
            RangeInclusive<Address>,
            Option<Address>,
            &ErasedComponentHandle,
        ) -> Result<(), MemoryError>,
    ) -> Result<(), MemoryError> {
        let start_page = access_range.start() / PAGE_SIZE;
        let end_page = access_range.end() / PAGE_SIZE;
        let mut was_handled = false;

        for page_index in start_page..=end_page {
            let page = &self.table[page_index];

            for entry in page
                .iter()
                .filter(|entry| access_range.intersects(&(entry.start..=entry.end)))
            {
                let entry_range = entry.start..=entry.end;

                was_handled = true;
                visitor(entry_range.clone(), entry.mirror_start, &entry.component)?;

                // If this range completely contains our accessing range we can exit early without more searching
                let cropped_access_range = entry_range.intersection(&access_range);

                if cropped_access_range == access_range {
                    return Ok(());
                }
            }
        }

        if !was_handled {
            return Err(MemoryError(
                std::iter::once((access_range, MemoryErrorType::Denied)).collect(),
            ));
        }

        Ok(())
    }
}
