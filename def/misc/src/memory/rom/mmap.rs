use crate::memory::rom::RomMemoryBackend;
use memmap2::Mmap;
use std::fs::File;

#[derive(Debug)]
pub struct MmapBackend(Mmap);

impl RomMemoryBackend for MmapBackend {
    fn new(file: File) -> Self {
        Self(unsafe { Mmap::map(&file).unwrap() })
    }

    fn read(&self, offset: usize, buffer: &mut [u8]) {
        buffer.copy_from_slice(&self.0[offset..offset + buffer.len()]);
    }
}
