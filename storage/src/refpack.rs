//! pack of references, in a certain order

use config::{StorageConfig};
use containers::reffile;
use super::Result;

pub use std::collections::vec_deque::{Iter};

pub fn read_refpack<P: AsRef<str>>(storage_config: &StorageConfig, name: P) -> Result<reffile::Lookup> {
    let r = reffile::Lookup::from_path(storage_config.get_refpack_filepath(name))?;
    Ok(r)
}

pub fn write_refpack<P: AsRef<str>>(storage_config: &StorageConfig, name: P, rf: &reffile::Lookup) -> Result<()> {
    let path = storage_config.get_refpack_filepath(name);
    rf.to_path(path)?;
    Ok(())
}
