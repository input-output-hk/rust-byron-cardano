//! pack of references, in a certain order

use super::Result;
use config::StorageConfig;
use storage_units::reffile;

pub use std::collections::vec_deque::Iter;

pub fn read_refpack<P: AsRef<str>>(
    storage_config: &StorageConfig,
    name: P,
) -> Result<reffile::Lookup> {
    let r = reffile::Lookup::from_path(storage_config.get_refpack_filepath(name))?;
    Ok(r)
}

pub fn write_refpack<P: AsRef<str>>(
    storage_config: &StorageConfig,
    name: P,
    rf: &reffile::Lookup,
) -> Result<()> {
    let path = storage_config.get_refpack_filepath(name);
    rf.to_path(path)?;
    Ok(())
}
