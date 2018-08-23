//! pack of references, in a certain order

use std::{io, result, fmt};
use config::{StorageConfig};
use containers::reffile;

pub use std::collections::vec_deque::{Iter};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::IoError(ref err) => write!(f, "IO Error: {}", err)
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

pub fn read_refpack<P: AsRef<str>>(storage_config: &StorageConfig, name: P) -> Result<reffile::Lookup> {
    let r = reffile::Lookup::from_path(storage_config.get_refpack_filepath(name))?;
    Ok(r)
}

pub fn write_refpack<P: AsRef<str>>(storage_config: &StorageConfig, name: P, rf: &reffile::Lookup) -> Result<()> {
    let path = storage_config.get_refpack_filepath(name);
    rf.to_path(path)?;
    Ok(())
}
