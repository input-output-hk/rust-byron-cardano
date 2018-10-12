#[macro_use]
extern crate log;
extern crate cbor_event;
extern crate storage_units;
extern crate cardano;
extern crate rand;

pub mod types;
pub mod config;
pub mod pack;
pub mod tag;
pub mod epoch;
pub mod refpack;
pub mod utxo;
pub mod iter;
use std::{fs, io, result};

pub use config::StorageConfig;

use std::{collections::BTreeMap, fmt, error};
use cardano::{block::{HeaderHash, BlockDate, RawBlock, Block, EpochId, SlotId}};

use types::*;
use storage_units::utils::tmpfile::*;
use storage_units::utils::magic;
use storage_units::utils::error::StorageError;

use storage_units::{packfile, indexfile, reffile};
use pack::{packreader_init, packreader_block_next};
use std::cmp::Ordering;

#[derive(Debug)]
pub enum Error {
    StorageError(StorageError),
    CborBlockError(cbor_event::Error),
    RefPackUnexpectedBoundary(SlotId),

    HashNotFound(HeaderHash),

    // ** Epoch pack assumption errors
    EpochExpectingBoundary,
    EpochError(EpochId, EpochId),
    EpochSlotRewind(EpochId, SlotId),
    EpochChainInvalid(BlockDate, HeaderHash, HeaderHash),
    NoSuchTag
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::StorageError(e.into()) }
}
impl From<StorageError> for Error {
    fn from(e: StorageError) -> Self { Error::StorageError(e) }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self { Error::CborBlockError(e) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::StorageError(_) => write!(f, "Storage error"),
            Error::CborBlockError(_) => write!(f, "Encoding error"),
            Error::HashNotFound(hh) => write!(f, "Hash not found {}", hh),
            Error::RefPackUnexpectedBoundary(sid) => write!(f, "Ref pack has an unexpected Boundary `{}`", sid),
            Error::EpochExpectingBoundary => write!(f, "Expected a boundary block"),
            Error::EpochError(eeid, reid) => write!(f, "Expected block in epoch {} but is in epoch {}", eeid, reid),
            Error::EpochSlotRewind(eid, sid) => write!(f, "Cannot pack block {} because is prior to {} already packed", sid, eid),
            Error::EpochChainInvalid(bd, rhh, ehh) => write!(f, "Cannot pack block {} ({}) because it does not follow the blockchain hash (expected: {})", bd, ehh, rhh),
            Error::NoSuchTag => write!(f, "Tag not found"),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::StorageError(ref err) => Some(err),
            Error::CborBlockError(ref err) => Some(err),
            Error::HashNotFound(_) => None,
            Error::RefPackUnexpectedBoundary(_) => None,
            Error::EpochExpectingBoundary => None,
            Error::EpochError(_, _) => None,
            Error::EpochSlotRewind(_, _) => None,
            Error::EpochChainInvalid(_, _, _) => None,
            Error::NoSuchTag => None
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

pub struct Storage {
    pub config: StorageConfig,
    lookups: BTreeMap<PackHash, indexfile::Lookup>,
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

    /// construct a range between the given hash
    pub fn range(&self, from: BlockHash, to: BlockHash) -> Result<iter::Range> {
        iter::Range::new(self, from, to)
    }

    pub fn get_block_from_tag(&self, tag: &str) -> Result<Block> {
        match tag::read_hash(&self, &tag) {
            None => Err(Error::NoSuchTag),
            Some(hash) => {
                match block_read(&self, &hash) {
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
    use std::io::{Read,Write};
    use super::{Result};
    use cardano::block::RawBlock;
    use magic;

    const FILE_TYPE: magic::FileType = 0x424c4f42; // = BLOB
    const VERSION: magic::Version = 1;

    pub fn write(storage: &super::Storage, hash: &super::BlockHash, block: &[u8]) -> Result<()> {
        let path = storage.config.get_blob_filepath(&hash);
        let mut tmp_file = super::tmpfile_create_type(storage, super::StorageFileType::Blob);
        magic::write_header(&mut tmp_file, FILE_TYPE, VERSION)?;
        tmp_file.write_all(block)?;
        tmp_file.render_permanent(&path)?;
        Ok(())
    }

    pub fn read_raw(storage: &super::Storage, hash: &super::BlockHash) -> Result<Vec<u8>> {
        let mut content = Vec::new();
        let path = storage.config.get_blob_filepath(&hash);

        let mut file = fs::File::open(path)?;
        magic::check_header(&mut file, FILE_TYPE, VERSION, VERSION)?;
        file.read_to_end(&mut content)?;
        Ok(content)
    }

    pub fn read(storage: &super::Storage, hash: &super::BlockHash) -> Result<RawBlock> {
        Ok(RawBlock::from_dat(self::read_raw(storage, hash)?))
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
    Packed(PackHash, indexfile::IndexOffset),
    Loose,
}

pub fn block_location(storage: &Storage, hash: &BlockHash) -> Option<BlockLocation> {
    for (packref, lookup) in storage.lookups.iter() {
        let (start, nb) = lookup.fanout.get_indexer_by_hash(hash);
        match nb {
            indexfile::FanoutNb(0) => {},
            _ => {
                if lookup.bloom.search(hash) {
                    let idx_filepath = storage.config.get_index_filepath(packref);
                    let mut idx_file = indexfile::Reader::init(idx_filepath).unwrap();
                    match idx_file.search(&lookup.params, hash, start, nb) {
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
                    let mut idx_file = indexfile::ReaderNoLookup::init(idx_filepath).unwrap();
                    let pack_offset = idx_file.resolve_index_offset(lookup, *iofs);
                    let pack_filepath = storage.config.get_pack_filepath(packref);
                    let mut pack_file = packfile::Seeker::init(pack_filepath).unwrap();
                    pack_file.get_at_offset(pack_offset).ok().and_then(|x| Some(RawBlock(x)))
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

enum ReverseSearch {
    Continue,
    Found,
    Abort,
}

fn previous_block(storage: &Storage, block: &Block) -> Block {
    let prev_hash = block.get_header().get_previous_header();
    let blk = blob::read(&storage, &header_to_blockhash(&prev_hash)).unwrap().decode().unwrap();
    blk
}

fn block_reverse_search_from_tip<F>(storage: &Storage, first_block: &Block, find: F) -> Result<Option<Block>>
    where F: Fn(&Block) -> Result<ReverseSearch>
{
    let mut current_blk = first_block.clone();
    loop {
        match find(&current_blk)? {
            ReverseSearch::Continue => {
                let blk = previous_block(&storage, &current_blk);
                current_blk = blk;
            },
            ReverseSearch::Found => { return Ok(Some(current_blk)) },
            ReverseSearch::Abort => { return Ok(None) },
        };
    }
}

pub fn resolve_date_to_blockhash(storage: &Storage, tip: &BlockHash, date: &BlockDate) -> Result<Option<BlockHash>> {
    let epoch = date.get_epochid();
    match epoch::epoch_open_packref(&storage.config, epoch) {
        Ok(mut handle) => {
            let slotid = match date {
                BlockDate::Boundary(_) => 0,
                BlockDate::Normal(sid) => sid.slotid,
            };
            let r = handle.getref_at_index(slotid as u32)?;
            Ok(r)
        },
        Err(_) => {
            let tip_rblk = block_read(&storage, tip);
            match tip_rblk {
                None => return Ok(None),
                Some(rblk) => {
                    let blk = rblk.decode()?;
                    let found = block_reverse_search_from_tip(storage, &blk, |x|
                        match x.get_header().get_blockdate().cmp(date) {
                            Ordering::Equal => Ok(ReverseSearch::Found),
                            Ordering::Greater => Ok(ReverseSearch::Continue),
                            Ordering::Less => Ok(ReverseSearch::Abort),
                        })?;
                    Ok(found.map(|x| header_to_blockhash(&x.get_header().compute_hash())))
                }
            }
        },
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
    let mut writer = pack::packwriter_init(&storage.config).unwrap();
    let mut blob_packed = Vec::new();

    let block_hashes : Vec<BlockHash> = if let Some((from, to)) = params.range {
        storage.range(from, to).unwrap().iter().cloned().collect()
    } else {
        storage.config.list_blob(params.limit_nb_blobs)
    };
    for bh in block_hashes {
        let blob = blob::read_raw(storage, &bh).unwrap();
        writer.append(&bh, &blob[..]).unwrap();
        blob_packed.push(bh);
        match params.limit_size {
            None => {},
            Some(sz) => {
                if writer.pos >= sz {
                    break
                }
            }
        }
    }

    let (packhash, index) = pack::packwriter_finalize(&storage.config, writer);

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
    let mut rp = reffile::Lookup::new();
    let packhash_vec = tag::read(storage, tag).expect("EPOCH not found");
    let mut packhash = [0;HASH_SIZE];
    packhash[..].clone_from_slice(packhash_vec.as_slice());
    let mut pack = packreader_init(&storage.config, &packhash);

    let mut current_state = None;

    while let Some(raw_block) = packreader_block_next(&mut pack) {
        let block = raw_block.decode()?;
        let hdr = block.get_header();
        let hash = hdr.compute_hash();
        let date = hdr.get_blockdate();

        // either we have seen genesis yet or not
        match current_state {
            None      => {
                if !hdr.is_boundary_block() {
                    return Err(Error::EpochExpectingBoundary)
                }
                current_state = Some((hdr.get_blockdate().get_epochid(), 0, hdr.compute_hash()));
                rp.append_hash(hash.into());
            },
            Some((current_epoch, expected_slotid, current_prevhash)) => {
                match date.clone() {
                    cardano::block::BlockDate::Boundary(_) => {
                        return Err(Error::RefPackUnexpectedBoundary(expected_slotid));
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
                            rp.append_missing_hash();
                            current_slotid += 1;
                        }
                        rp.append_hash(hash.clone().into());
                        current_state = Some((current_epoch, current_slotid, hash));
                    },
                }
            },
        }
    }

    refpack::write_refpack(&storage.config, tag, &rp).map_err(From::from)
}

pub fn integrity_check(storage: &Storage, genesis_hash: HeaderHash, count: EpochId) {
    let mut previous_header = genesis_hash;
    for epochid in 0..count {
        println!("check epoch {}'s integrity", epochid);
        previous_header = epoch_integrity_check(storage, epochid, previous_header).unwrap();
    }
}

// FIXME: still necessary now that we have verify_block?
fn epoch_integrity_check(storage: &Storage, epochid: EpochId, last_known_hash: HeaderHash) -> Result<HeaderHash> {
    let packhash_vec = tag::read(storage, &format!("EPOCH_{}", epochid)).expect("EPOCH not found");
    let mut packhash = [0;HASH_SIZE];
    packhash[..].clone_from_slice(packhash_vec.as_slice());
    let mut pack = packreader_init(&storage.config, &packhash);

    let mut current_state = None;

    while let Some(raw_block) = packreader_block_next(&mut pack) {
        let block = raw_block.decode()?;
        let hdr = block.get_header();
        let hash = hdr.compute_hash();
        let prevhash = hdr.get_previous_header();
        let date = hdr.get_blockdate();

        // either we have seen genesis yet or not
        match current_state {
            None      => {
                if !hdr.is_boundary_block() {
                    return Err(Error::EpochExpectingBoundary)
                }
                if last_known_hash != prevhash {
                    return Err(Error::EpochChainInvalid(date, last_known_hash, prevhash))
                }
                current_state = Some((hdr.get_blockdate().get_epochid(), 0, hdr.compute_hash()));
            },
            Some((current_epoch, expected_slotid, current_prevhash)) => {
                match date.clone() {
                    cardano::block::BlockDate::Boundary(_) => {
                        return Err(Error::RefPackUnexpectedBoundary(expected_slotid));
                    },
                    cardano::block::BlockDate::Normal(slotid) => {
                        if slotid.epoch != current_epoch {
                            return Err(Error::EpochError(current_epoch, slotid.epoch));
                        }
                        if slotid.slotid < expected_slotid {
                            return Err(Error::EpochSlotRewind(current_epoch, slotid.slotid));
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
