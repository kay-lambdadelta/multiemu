use std::io::{Read, Seek, Write};

use nod::{
    common::Format,
    read::{DiscOptions, DiscReader, PartitionEncryption},
    write::{DiscWriter, FormatOptions, ProcessOptions, ScrubLevel},
};

pub fn to_iso(
    rom: impl Read + Seek + Send + 'static,
) -> Result<impl Read + Seek, Box<dyn std::error::Error + Send + Sync>> {
    let mut tempfile = tempfile::tempfile()?;
    let reader = DiscReader::new_from_non_cloneable_read(
        rom,
        &DiscOptions {
            partition_encryption: PartitionEncryption::Original,
            preloader_threads: 0,
        },
    )?;

    let writer = DiscWriter::new(
        reader,
        &FormatOptions {
            format: Format::Iso,
            compression: Format::Iso.default_compression(),
            block_size: Format::Iso.default_block_size(),
        },
    )?;

    writer.process(
        |bytes, _, _| {
            tempfile.write_all(bytes.as_ref())?;

            Ok(())
        },
        &ProcessOptions {
            processor_threads: num_cpus::get(),
            digest_crc32: false,
            digest_md5: false,
            digest_sha1: false,
            digest_xxh64: false,
            scrub: ScrubLevel::None,
        },
    )?;
    tempfile.rewind()?;

    Ok(tempfile)
}
