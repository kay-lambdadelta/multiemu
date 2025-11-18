use std::{fs::File, io::Read};

use bytes::Bytes;

use crate::memory::rom::RomMemoryBackend;

#[derive(Debug)]
pub struct VecBackend;

impl RomMemoryBackend for VecBackend {
    fn open(mut file: File) -> Bytes {
        let mut buffer = Vec::default();
        file.read_to_end(&mut buffer);

        Bytes::from_owner(buffer)
    }
}
