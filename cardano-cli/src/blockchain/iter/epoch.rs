//! module to iterator through a given epoch
//!
//! The iterator stops once the epoch is completely traversed.
//!

use cardano::block::{EpochId, RawBlock};
use storage::{StorageConfig, epoch::{epoch_read_pack}, pack::{PackReader}};

use std::{fs};

use super::{Result, Error};

/// Iterator over every blocks of a given epoch
pub struct Iter(PackReader<fs::File>);
impl Iter {
    pub fn new(storage: &StorageConfig, epoch: EpochId) -> Result<Self> {
        let packref = epoch_read_pack(storage, epoch)?;
        let reader = PackReader::init(&storage, &packref);
        Ok(Iter(reader))
    }

    // TODO:
    // * pub fn from_hash(mut self, hash: HeaderHash) -> Self {}
    // * pub fn from_slot(mut self, slot: u32) -> Self;
}
impl Iterator for Iter {
    type Item = Result<RawBlock>;
    fn next(&mut self) -> Option<Self::Item> {
        // TODO, this is dodgy, there should be a Result to get from it
        //       should have at least `::io::Error` as we are using
        //       PackReader<**fs::File**>;
        self.0.get_next().map(|raw_block| Ok(raw_block))
    }
}

/// Create an iterator over every epoch of the storage
///
/// The iterator will returns `Iter` so it is possible to iterate
/// over the block from it.
pub struct Epochs<'a> {
    storage_config: &'a StorageConfig,

    epoch_id: EpochId
}
impl<'a> Epochs<'a> {
    pub fn new(storage: &'a StorageConfig) -> Self {
        Epochs { storage_config: storage, epoch_id: 0 }
    }

    pub fn from_epoch(mut self, epoch_id: EpochId) -> Self {
        self.epoch_id = epoch_id;
        self
    }
}
impl<'a> Iterator for Epochs<'a> {
    type Item = Result<Iter>;

    fn next(&mut self) -> Option<Self::Item> {
        match epoch_read_pack(&self.storage_config, self.epoch_id) {
            Err(err) => {
                if err.kind() == ::std::io::ErrorKind::NotFound {
                    None
                } else {
                    Some(Err(Error::IoError(err)))
                }
            },
            Ok(epoch_ref) => {
                let iter = Iter(PackReader::init(&self.storage_config, &epoch_ref));
                self.epoch_id += 1;
                Some(Ok(iter))
            }
        }
    }
}
