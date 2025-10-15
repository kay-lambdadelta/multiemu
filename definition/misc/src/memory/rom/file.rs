use crate::memory::rom::RomMemoryBackend;
use std::fs::File;
#[cfg(not(target_family = "unix"))]
use std::sync::Mutex;

#[cfg(not(target_family = "unix"))]
#[derive(Debug)]
pub struct FileBackend(Mutex<File>);

#[cfg(target_family = "unix")]
#[derive(Debug)]
pub struct FileBackend(File);

impl RomMemoryBackend for FileBackend {
    fn new(file: File) -> Self {
        #[cfg(target_family = "unix")]
        return FileBackend(file);

        #[cfg(not(target_family = "unix"))]
        return FileBackend(Mutex::new(file));
    }

    #[inline]
    #[cfg(not(target_family = "unix"))]
    fn read(&self, offset: usize, buffer: &mut [u8]) {
        use std::io::{Read, Seek, SeekFrom};

        let mut file_guard = self.0.lock().unwrap();
        file_guard.seek(SeekFrom::Start(offset as u64)).unwrap();
        file_guard.read_exact(buffer).unwrap();
    }

    #[inline]
    #[cfg(target_family = "unix")]
    fn read(&self, offset: usize, buffer: &mut [u8]) {
        use std::os::unix::fs::FileExt;

        self.0.read_exact_at(buffer, offset as u64).unwrap();
    }
}
