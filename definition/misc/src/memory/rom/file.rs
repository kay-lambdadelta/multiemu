use crate::memory::rom::RomMemoryBackend;
use std::fs::File;
#[cfg(not(unix))]
use std::sync::Mutex;

#[cfg(not(unix))]
#[derive(Debug)]
pub struct FileBackend(Mutex<File>);

#[cfg(unix)]
#[derive(Debug)]
pub struct FileBackend(File);

impl RomMemoryBackend for FileBackend {
    fn new(file: File) -> Self {
        #[cfg(unix)]
        return FileBackend(file);

        #[cfg(not(unix))]
        return FileBackend(Mutex::new(file));
    }

    #[inline]
    #[cfg(not(unix))]
    fn read(&self, offset: usize, buffer: &mut [u8]) {
        use std::io::{Read, Seek, SeekFrom};

        let mut file_guard = self.0.lock().unwrap();
        file_guard.seek(SeekFrom::Start(offset as u64)).unwrap();
        file_guard.read_exact(buffer).unwrap();
    }

    #[inline]
    #[cfg(unix)]
    fn read(&self, offset: usize, buffer: &mut [u8]) {
        use std::os::unix::fs::FileExt;

        self.0.read_exact_at(buffer, offset as u64).unwrap();
    }
}
