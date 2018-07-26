#[macro_use]
extern crate log;
extern crate cryptoxide;
extern crate cbor_event;
extern crate cardano;
extern crate rand;
extern crate flate2;

pub mod block;
pub mod types;
pub mod config;
pub mod pack;
pub mod tag;
pub mod epoch;
pub mod refpack;
pub mod tmpfile;
pub mod lock;
pub mod append;
mod compression;
mod bitmap;
mod bloom;
use std::{fs, io, result};

pub use config::StorageConfig;

use std::collections::BTreeMap;
use refpack::{RefPack};
use cardano::block::{HeaderHash, BlockDate, RawBlock, Block};

use types::*;
use tmpfile::*;

const USE_COMPRESSION : bool = true;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    BlockError(block::Error),
    CborBlockError(cbor_event::Error),
    // ** RefPack creation errors
    RefPackError(refpack::Error),
    RefPackUnexpectedGenesis(u32),
    // ** Epoch pack assumption errors
    EpochExpectingGenesis,
    EpochError(u32, u32),
    EpochSlotRewind(u32, u32),
    EpochChainInvalid(BlockDate, HeaderHash, HeaderHash),
    NoSuchTag
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<block::Error> for Error {
    fn from(e: block::Error) -> Self { Error::BlockError(e) }
}
impl From<refpack::Error> for Error {
    fn from(e: refpack::Error) -> Self { Error::RefPackError(e) }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self { Error::CborBlockError(e) }
}

pub type Result<T> = result::Result<T, Error>;

pub struct Storage {
    pub config: StorageConfig,
    lookups: BTreeMap<PackHash, pack::Lookup>,
}

impl Storage {
    pub fn init(cfg: &StorageConfig) -> Result<Self> {
        let mut lookups = BTreeMap::new();

        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Blob))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Index))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Pack))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Tag))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Epoch))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::RefPack))?;

        let packhashes = cfg.list_indexes();
        for p in packhashes.iter() {
            match pack::read_index_fanout(&cfg, p) {
                Err(_)     => {},
                Ok(lookup) => {
                    lookups.insert(*p, lookup);
                }
            }
        }

        let storage = Storage { config: cfg.clone(), lookups: lookups };
        Ok(storage)
    }

    /// create a reverse iterator over the stored blocks
    ///
    /// it will iterate from the tag `HEAD` until there is no more
    /// value to parse
    pub fn reverse_iter<'a>(&'a self) -> Result<block::ReverseIter<'a>> {
        block::ReverseIter::new(self).map_err(|err| Error::BlockError(err))
    }

    /// create a block iterator starting from the given EpochId
    pub fn iterate_from_epoch<'a>(&'a self, from: cardano::block::EpochId) -> Result<block::Iter<'a>> {
        Ok(block::Iter::new(&self.config, from)?)
    }

    /// construct a range between the given hash
    pub fn range(&self, from: BlockHash, to: BlockHash) -> Result<block::Range> {
        block::Range::new(self, from, to).map_err(|err| Error::BlockError(err))
    }

    pub fn get_block_from_tag(&self, tag: &str) -> Result<Block> {
        match tag::read_hash(&self, &tag) {
            None => Err(Error::NoSuchTag),
            Some(hash) => {
                match block_read(&self, hash.bytes()) {
                    None => {
                        warn!("tag '{}' refers to non-existent block {}", tag, hash);
                        Err(Error::NoSuchTag)
                    },
                    Some(block) => Ok(block.decode()?)
                }
            }
        }
    }
}

fn tmpfile_create_type(storage: &Storage, filetype: StorageFileType) -> TmpFile {
    TmpFile::create(storage.config.get_filetype_dir(filetype)).unwrap()
}

pub mod blob {
    use std::fs;
    use std::io::{Read};
    use super::{Result, Error};
    use compression;
    use cardano::block::RawBlock;

    pub fn write(storage: &super::Storage, hash: &super::BlockHash, block: &[u8]) -> Result<()> {
        let path = storage.config.get_blob_filepath(&hash);
        let mut tmp_file = super::tmpfile_create_type(storage, super::StorageFileType::Blob);
        compression::compress_write(&mut tmp_file, block)?;
        tmp_file.render_permanent(&path).map_err(|e| Error::IoError(e))
    }

    pub fn read_raw(storage: &super::Storage, hash: &super::BlockHash) -> Result<Vec<u8>> {
        let mut content = Vec::new();
        let path = storage.config.get_blob_filepath(&hash);

        let mut file = fs::File::open(path)?;
        file.read_to_end(&mut content)?;
        Ok(content)
    }

    pub fn read(storage: &super::Storage, hash: &super::BlockHash) -> Result<RawBlock> {
        let mut content = Vec::new();
        let path = storage.config.get_blob_filepath(&hash);

        let mut file = fs::File::open(path)?;
        file.read_to_end(&mut content)?;

        Ok(RawBlock::from_dat(compression::decompress_conditional(content.as_ref())))
    }

    pub fn exist(storage: &super::Storage, hash: &super::BlockHash) -> bool {
        let p = storage.config.get_blob_filepath(hash);
        p.as_path().exists()
    }

    pub fn remove(storage: &super::Storage, hash: &super::BlockHash) {
        let p = storage.config.get_blob_filepath(hash);
        match fs::remove_file(p) {
            Ok(()) => {},
            Err(_) => {},
        }
    }
}

#[derive(Clone, Debug)]
pub enum BlockLocation {
    Packed(PackHash, pack::IndexOffset),
    Loose,
}

pub fn block_location(storage: &Storage, hash: &BlockHash) -> Option<BlockLocation> {
    for (packref, lookup) in storage.lookups.iter() {
        let (start, nb) = lookup.fanout.get_indexer_by_hash(hash);
        match nb {
            pack::FanoutNb(0) => {},
            _                 => {
                if lookup.bloom.search(hash) {
                    let idx_filepath = storage.config.get_index_filepath(packref);
                    let mut idx_file = fs::File::open(idx_filepath).unwrap();
                    match pack::search_index(&mut idx_file, &lookup.params, hash, start, nb) {
                        None       => {},
                        Some(iloc) => return Some(BlockLocation::Packed(packref.clone(), iloc)),
                    }
                }
            }
        }
    }
    if blob::exist(storage, hash) {
        return Some(BlockLocation::Loose);
    }
    None
}

pub fn block_read_location(storage: &Storage, loc: &BlockLocation, hash: &BlockHash) -> Option<RawBlock> {
    match loc {
        &BlockLocation::Loose                 => blob::read(storage, hash).ok(),
        &BlockLocation::Packed(ref packref, ref iofs) => {
            match storage.lookups.get(packref) {
                None         => { unreachable!(); },
                Some(lookup) => {
                    let idx_filepath = storage.config.get_index_filepath(packref);
                    let mut idx_file = fs::File::open(idx_filepath).unwrap();
                    let pack_offset = pack::resolve_index_offset(&mut idx_file, lookup, *iofs);
                    let pack_filepath = storage.config.get_pack_filepath(packref);
                    let mut pack_file = fs::File::open(pack_filepath).unwrap();
                    pack::read_block_at(&mut pack_file, pack_offset).ok()
                }
            }
        }
    }
}

pub fn block_read(storage: &Storage, hash: &BlockHash) -> Option<RawBlock> {
    match block_location(storage, hash) {
        None      => None,
        Some(loc) => block_read_location(storage, &loc, hash),
    }
}

/// packing parameters
///
/// optionally set the maximum number of blobs in this pack
/// optionally set the maximum size in bytes of the pack file.
///            note that the limits is best effort, not strict.
pub struct PackParameters {
    pub limit_nb_blobs: Option<u32>,
    pub limit_size: Option<u64>,
    pub delete_blobs_after_pack: bool,
    pub range: Option<(BlockHash, BlockHash)>,
}
impl Default for PackParameters {
    fn default() -> Self {
        PackParameters {
            limit_nb_blobs: None,
            limit_size: None,
            delete_blobs_after_pack: true,
            range: None,
        }
    }
}

pub fn pack_blobs(storage: &mut Storage, params: &PackParameters) -> PackHash {
    let mut writer = pack::PackWriter::init(&storage.config);
    let mut blob_packed = Vec::new();

    let block_hashes : Vec<BlockHash> = if let Some((from, to)) = params.range {
        storage.range(from, to).unwrap().iter().cloned().collect()
    } else {
        storage.config.list_blob(params.limit_nb_blobs)
    };
    for bh in block_hashes {
        let blob = blob::read_raw(storage, &bh).unwrap();
        writer.append(&bh, &blob[..]);
        blob_packed.push(bh);
        match params.limit_size {
            None => {},
            Some(sz) => {
                if writer.get_current_size() >= sz {
                    break
                }
            }
        }
    }

    let (packhash, index) = writer.finalize();

    let (lookup, tmpfile) = pack::create_index(storage, &index);
    tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();

    if params.delete_blobs_after_pack {
        for bh in blob_packed.iter() {
            blob::remove(storage, bh);
        }
    }

    // append to lookups
    storage.lookups.insert(packhash, lookup);
    packhash
}

// Create a pack of references (packref) of all the hash in an epoch pack
//
// If the pack is not valid, then an error is returned
pub fn refpack_epoch_pack<S: AsRef<str>>(storage: &Storage, tag: &S) -> Result<()> {
    let mut rp = RefPack::new();
    let packhash_vec = tag::read(storage, tag).expect("EPOCH not found");
    let mut packhash = [0;HASH_SIZE];
    packhash[..].clone_from_slice(packhash_vec.as_slice());
    let mut pack = pack::PackReader::init(&storage.config, &packhash);

    let mut current_state = None;

    while let Some(raw_block) = pack.get_next() {
        let block = raw_block.decode()?;
        let hdr = block.get_header();
        let hash = hdr.compute_hash();
        let date = hdr.get_blockdate();

        // either we have seen genesis yet or not
        match current_state {
            None      => {
                if !hdr.is_genesis_block() {
                    return Err(Error::EpochExpectingGenesis)
                }
                current_state = Some((hdr.get_blockdate().get_epochid(), 0, hdr.compute_hash()));
                rp.push_back(hash.into_bytes());
            },
            Some((current_epoch, expected_slotid, current_prevhash)) => {
                match date.clone() {
                    cardano::block::BlockDate::Genesis(_) => {
                        return Err(Error::RefPackUnexpectedGenesis(expected_slotid));
                    },
                    cardano::block::BlockDate::Normal(ref slotid) => {
                        if slotid.epoch != current_epoch {
                            return Err(Error::EpochError(current_epoch, slotid.epoch));
                        }
                        if slotid.slotid < expected_slotid {
                            return Err(Error::EpochSlotRewind(current_epoch, slotid.slotid));
                        }
                        if hdr.get_previous_header() != current_prevhash {
                            return Err(Error::EpochChainInvalid(date, hdr.get_previous_header(), current_prevhash))
                        }

                        let mut current_slotid = expected_slotid;

                        while current_slotid < slotid.slotid {
                            rp.push_back_missing();
                            current_slotid += 1;
                        }
                        rp.push_back(hash.clone().into_bytes());
                        current_state = Some((current_epoch, current_slotid, hash));
                    },
                }
            },
        }
    }

    refpack::write_refpack(&storage.config, tag, &rp).map_err(From::from)
}

pub fn integrity_check(storage: &Storage, genesis_hash: HeaderHash, count: u32) {
    let mut previous_header = genesis_hash;
    for epochid in 0..count {
        println!("check epoch {}'s integrity", epochid);
        previous_header = epoch_integrity_check(storage, epochid, previous_header).unwrap();
    }
}

fn epoch_integrity_check(storage: &Storage, epochid: u32, last_known_hash: HeaderHash) -> Result<HeaderHash> {
    let packhash_vec = tag::read(storage, &format!("EPOCH_{}", epochid)).expect("EPOCH not found");
    let mut packhash = [0;HASH_SIZE];
    packhash[..].clone_from_slice(packhash_vec.as_slice());
    let mut pack = pack::PackReader::init(&storage.config, &packhash);

    let mut current_state = None;

    while let Some(raw_block) = pack.get_next() {
        let block = raw_block.decode()?;
        let hdr = block.get_header();
        let hash = hdr.compute_hash();
        let prevhash = hdr.get_previous_header();
        let date = hdr.get_blockdate();

        // either we have seen genesis yet or not
        match current_state {
            None      => {
                if !hdr.is_genesis_block() {
                    return Err(Error::EpochExpectingGenesis)
                }
                if last_known_hash != prevhash {
                    return Err(Error::EpochChainInvalid(date, last_known_hash, prevhash))
                }
                current_state = Some((hdr.get_blockdate().get_epochid(), 0, hdr.compute_hash()));
            },
            Some((current_epoch, expected_slotid, current_prevhash)) => {
                match date.clone() {
                    cardano::block::BlockDate::Genesis(_) => {
                        return Err(Error::RefPackUnexpectedGenesis(expected_slotid));
                    },
                    cardano::block::BlockDate::Normal(slotid) => {
                        if slotid.epoch != current_epoch {
                            return Err(Error::EpochError(current_epoch, slotid.epoch));
                        }
                        if slotid.slotid < expected_slotid {
                            return Err(Error::EpochSlotRewind(current_epoch, slotid.epoch));
                        }
                        if prevhash != current_prevhash {
                            return Err(Error::EpochChainInvalid(date.clone(), prevhash, current_prevhash))
                        }

                        let mut current_slotid = expected_slotid;

                        while current_slotid < slotid.slotid {
                            current_slotid += 1;
                        }
                        current_state = Some((current_epoch, current_slotid, hash));
                    },
                }
            },
        }
    }
    match current_state {
        None => { panic!("test") },
        Some((_, _, prevhash)) => {
            return Ok(prevhash)
        }
    }
}
