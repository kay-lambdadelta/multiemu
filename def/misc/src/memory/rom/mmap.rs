use crate::memory::rom::RomMemoryBackend;
use bytes::Bytes;
use memmap2::Mmap;
use multiemu::memory::Address;
use std::fs::File;

#[derive(Debug)]
pub struct MmapBackend(Bytes);

impl RomMemoryBackend for MmapBackend {
    fn new(file: File) -> Self {
        Self(Bytes::from_owner(unsafe { Mmap::map(&file).unwrap() }))
    }

    #[inline]
    fn read(&self, offset: Address, buffer: &mut [u8]) {
        buffer.copy_from_slice(&self.0[offset..offset + buffer.len()]);
    }

    fn get_bytes(&self) -> Option<Bytes> {
        Some(self.0.clone())
    }
}
