use std::io::{Write, Read};
use super::error::{Result, StorageError};
use utils::serialize::{read_size, write_size};

const MAGIC: &[u8;8] = b"\xfeCARDANO";
const MAGIC_SIZE: usize = 8;
const TYPE_SIZE: usize = 4;
const VERSION_SIZE: usize = 4;
pub const HEADER_SIZE: usize = MAGIC_SIZE + TYPE_SIZE + VERSION_SIZE;

pub type FileType = u32;
pub type Version = u32;

/// Write a 16-byte header consisting of a magic value, a file type,
/// and a file schema version number.
pub fn write_header<File>(
    file: &mut File,
    file_type: FileType,
    version: Version)
    -> Result<()>
    where File: Write
{
    let mut hdr_buf = [0u8;HEADER_SIZE];
    hdr_buf[0..8].clone_from_slice(&MAGIC[..]);
    write_size(&mut hdr_buf[8..12], file_type);
    write_size(&mut hdr_buf[12..16], version);
    file.write_all(&hdr_buf)?;
    Ok(())
}

/// Check that a file has a header denoting the expected file type and
/// has a version in the specified range. Return the version.
pub fn check_header(
    file: &mut Read,
    expected_file_type: FileType,
    min_version: Version,
    max_version: Version)
    -> Result<Version>
{
    let mut hdr_buf = [0u8;HEADER_SIZE];
    file.read_exact(&mut hdr_buf)?;

    if &hdr_buf[0..MAGIC_SIZE] != MAGIC {
        return Err(StorageError::MissingMagic);
    }

    let file_type = read_size(&hdr_buf[8..12]);
    let version = read_size(&hdr_buf[12..16]);

    if file_type != expected_file_type {
        return Err(StorageError::WrongFileType(expected_file_type, file_type));
    }

    if version < min_version {
        return Err(StorageError::VersionTooOld(min_version, version));
    }

    if version > max_version {
        return Err(StorageError::VersionTooNew(max_version, version));
    }

    Ok(version)
}

