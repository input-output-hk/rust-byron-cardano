use utils::magic;
use utils::lock;
use std::io;
use std::result;

/// Unified storage IO errors, with a set of common ones
#[derive(Debug)]
pub enum StorageError {
    IoError(io::Error),
    MissingMagic,
    WrongFileType(magic::FileType, magic::FileType),
    VersionTooOld(magic::Version, magic::Version),
    VersionTooNew(magic::Version, magic::Version),
    LockError(lock::Error),
}

impl From<io::Error> for StorageError {
    fn from(e: io::Error) -> Self { StorageError::IoError(e) }
}

pub type Result<T> = result::Result<T, StorageError>;