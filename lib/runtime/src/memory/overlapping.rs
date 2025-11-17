use std::ops::RangeInclusive;

use multiemu_range::RangeIntersection;

use crate::{
    component::ComponentHandle,
    memory::{Address, MemoryMappingTable, PAGE_SIZE},
};

pub struct OverlappingMappingsIter<'a> {
    table: &'a MemoryMappingTable,
    access_range: RangeInclusive<Address>,
    page_index: usize,
    end_page: usize,
    entry_index: usize,
}

impl MemoryMappingTable {
    #[inline]
    pub fn overlapping<'a>(
        &'a self,
        access_range: RangeInclusive<Address>,
    ) -> OverlappingMappingsIter<'a> {
        let start_page = access_range.start() / PAGE_SIZE;
        let end_page = access_range.end() / PAGE_SIZE;

        OverlappingMappingsIter {
            table: self,
            access_range,
            page_index: start_page,
            end_page,
            entry_index: 0,
        }
    }
}

pub struct Item<'a> {
    pub entry_assigned_range: RangeInclusive<Address>,
    pub mirror_start: Option<Address>,
    pub component: &'a ComponentHandle,
}

impl<'a> Iterator for OverlappingMappingsIter<'a> {
    type Item = Item<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.page_index <= self.end_page {
            let page = &self.table.table[self.page_index];

            while self.entry_index < page.len() {
                let entry = &page[self.entry_index];
                let entry_assigned_range = entry.start..=entry.end;

                self.entry_index += 1;

                if self.access_range.intersects(&entry_assigned_range) {
                    return Some(Item {
                        entry_assigned_range,
                        mirror_start: entry.mirror_start,
                        component: &entry.component,
                    });
                }
            }

            // move to next page
            self.page_index += 1;
            self.entry_index = 0;
        }

        None
    }
}
