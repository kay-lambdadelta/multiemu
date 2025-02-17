use nod::{Disc, OpenOptions};
use std::{
    fs::File,
    io::{Read, Seek},
    sync::Arc,
};
use tempfile::tempfile;

pub fn to_iso(
    // TODO: Once DynClone requirement is removed, change this to be generic
    file: File,
) -> Result<impl Read + Seek, Box<dyn std::error::Error + Send + Sync>> {
    let mut temp_file = tempfile()?;

    let mut disk = Disc::new_stream_with_options(
        Box::new(Arc::new(file)),
        &OpenOptions {
            rebuild_encryption: true,
            validate_hashes: true,
        },
    )?;

    std::io::copy(&mut disk, &mut temp_file)?;
    temp_file.rewind()?;

    Ok(temp_file)
}
