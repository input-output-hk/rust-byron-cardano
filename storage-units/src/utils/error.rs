use std::{error, fmt, io, result};
use utils::directory_name::DirectoryNameError;
use utils::lock;
use utils::magic;

/// Unified storage IO errors, with a set of common ones
#[derive(Debug)]
pub enum StorageError {
    IoError(io::Error),
    MissingMagic,
    WrongFileType(magic::FileType, magic::FileType),
    VersionTooOld(magic::Version, magic::Version),
    VersionTooNew(magic::Version, magic::Version),
    InvalidDirectoryName(DirectoryNameError),
    LockError(lock::Error),
}

impl From<io::Error> for StorageError {
    fn from(e: io::Error) -> Self {
        StorageError::IoError(e)
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StorageError::IoError(_) => write!(f, "I/O Error"),
            StorageError::MissingMagic => write!(f, "Missing storage file Magic bytes"),
            StorageError::WrongFileType(ftexpected, ftreceived) => write!(
                f,
                "Wrong file type, expected `0x{:04x}` but received `{:04x}`",
                ftexpected, ftreceived
            ),
            StorageError::VersionTooOld(mv, v) => write!(
                f,
                "File version is too old, supported at least `{}` but received `{}`",
                mv, v
            ),
            StorageError::VersionTooNew(mv, v) => write!(
                f,
                "File version is not supported yet, supported at most `{}` but received `{}`",
                mv, v
            ),
            StorageError::InvalidDirectoryName(_) => write!(f, "Invalid Directory name"),
            StorageError::LockError(_) => write!(f, "Lock file error"),
        }
    }
}

impl error::Error for StorageError {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            StorageError::IoError(ref err) => Some(err),
            StorageError::MissingMagic => None,
            StorageError::WrongFileType(_, _) => None,
            StorageError::VersionTooOld(_, _) => None,
            StorageError::VersionTooNew(_, _) => None,
            StorageError::InvalidDirectoryName(ref err) => Some(err),
            StorageError::LockError(ref err) => Some(err),
        }
    }
}

pub type Result<T> = result::Result<T, StorageError>;
