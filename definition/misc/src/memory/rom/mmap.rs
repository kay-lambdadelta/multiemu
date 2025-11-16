use std::fs::File;

use memmap2::Mmap;
use multiemu_runtime::memory::Address;

use crate::memory::rom::RomMemoryBackend;

#[derive(Debug)]
pub struct MmapBackend(Mmap);

impl RomMemoryBackend for MmapBackend {
    fn new(file: File) -> Self {
        // Modifying files on disk that are memmapped is UB, so we attempt to acquire a file lock to prevent such a thing
        let _ = file.lock_shared();

        Self(unsafe { Mmap::map(&file).unwrap() })
    }

    #[inline]
    fn read(&self, offset: Address, buffer: &mut [u8]) {
        buffer.copy_from_slice(&self.0[offset..offset + buffer.len()]);
    }
}
