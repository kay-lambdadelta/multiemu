use crate::memory::rom::RomMemoryBackend;
use bytes::Bytes;
use memmap2::Mmap;
use std::fs::File;

#[derive(Debug)]
pub struct MmapBackend;

impl RomMemoryBackend for MmapBackend {
    fn open(file: File) -> Bytes {
        // Modifying files on disk that are memmapped is UB, so we attempt to acquire a file lock to prevent such a thing
        let _ = file.lock_shared();
        let buffer = unsafe { Mmap::map(&file) }.unwrap();

        Bytes::from_owner(buffer)
    }
}
