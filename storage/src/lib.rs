#[macro_use]
extern crate log;
extern crate cardano;
extern crate cbor_event;
extern crate rand;
extern crate storage_units;
extern crate linked_hash_map;

pub mod chain_state;
pub mod config;
pub mod epoch;
pub mod iter;
pub mod pack;
pub mod refpack;
pub mod tag;
pub mod types;
use std::{fs, io, result};

pub use config::StorageConfig;

use cardano::block::{Block, BlockDate, EpochId, HeaderHash, RawBlock, SlotId};
use std::{error, fmt};
use linked_hash_map::LinkedHashMap;

use storage_units::utils::error::StorageError;
use storage_units::utils::{magic, serialize};
use storage_units::utils::tmpfile::*;
use types::*;

use pack::{packreader_block_next, packreader_init};
use std::cmp::Ordering;
use storage_units::{indexfile, packfile, reffile};
use cardano::util::hex;
use cardano::block::types::ChainDifficulty;
use cardano::block::block::BlockHeaderView;

#[derive(Debug)]
pub enum Error {
    StorageError(StorageError),
    CborBlockError(cbor_event::Error),
    BlockError(cardano::block::Error),

    RefPackUnexpectedBoundary(SlotId),

    BlockNotFound(BlockHash),
    BlockHeightNotFound(u64),

    // ** Epoch pack assumption errors
    EpochExpectingBoundary,
    EpochError(EpochId, EpochId),
    EpochSlotRewind(EpochId, SlotId),
    EpochChainInvalid(BlockDate, HeaderHash, HeaderHash),
    NoSuchTag,
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::StorageError(e.into())
    }
}
impl From<StorageError> for Error {
    fn from(e: StorageError) -> Self {
        Error::StorageError(e)
    }
}
impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self {
        Error::CborBlockError(e)
    }
}
impl From<cardano::block::Error> for Error {
    fn from(e: cardano::block::Error) -> Self {
        Error::BlockError(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::StorageError(_) => write!(f, "Storage error"),
            Error::CborBlockError(_) => write!(f, "Encoding error"),
            Error::BlockError(_) => write!(f, "Block error"),
            Error::BlockNotFound(hh) => write!(f, "Block {:?} not found", hex::encode(hh)),
            Error::BlockHeightNotFound(hh) => write!(f, "Block with height {:?} not found", hh),
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
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::StorageError(ref err) => Some(err),
            Error::CborBlockError(ref err) => Some(err),
            Error::BlockError(ref err) => Some(err),
            Error::BlockNotFound(_) => None,
            Error::BlockHeightNotFound(_) => None,
            Error::RefPackUnexpectedBoundary(_) => None,
            Error::EpochExpectingBoundary => None,
            Error::EpochError(_, _) => None,
            Error::EpochSlotRewind(_, _) => None,
            Error::EpochChainInvalid(_, _, _) => None,
            Error::NoSuchTag => None,
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

pub struct Storage {
    pub config: StorageConfig,
    lookups: LinkedHashMap<PackHash, indexfile::Lookup>,
    loose_idx: Vec<(ChainDifficulty, BlockDate, BlockHash)>,
}

macro_rules! try_open {
    ($open_fn:path, $path:expr, $what:expr) => {{
        let filepath = $path;
        match $open_fn(filepath) {
            Ok(file) => file,
            Err(e) => {
                warn!(
                    "cannot read {} `{}': {}",
                    $what,
                    filepath.to_string_lossy(),
                    e
                );
                return Err(e.into());
            }
        }
    }};
}

impl Storage {
    pub fn init(cfg: &StorageConfig) -> Result<Self> {
        let mut lookups = LinkedHashMap::with_capacity(1000);

        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Blob))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Index))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Pack))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Tag))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Epoch))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::RefPack))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::ChainState))?;

        let mut parsed_lookups = cfg.list_indexes().into_iter()
            .map(|p| (p, pack::read_index_fanout(&cfg, &p)))
            .filter_map(|(p, r)| match r {
                Ok(l) => Some((p, l)),
                _ => None,
            })
            .collect::<Vec<(PackHash, indexfile::Lookup)>>();

        parsed_lookups.sort_by(|a, b| a.1.params.ordinal.cmp(&b.1.params.ordinal));
        for (p, l) in parsed_lookups {
            lookups.insert(p, l);
        }

        let mut storage = Storage {
            config: cfg.clone(),
            lookups: lookups,
            loose_idx: vec![],
        };

        if let Some(hash) = tag::read_hash(&storage, &tag::HEAD) {
            storage.build_loose_index(hash);
        }

        Ok(storage)
    }

    fn build_loose_index(&mut self, tip: HeaderHash) {
        let mut cur_hash: HeaderHash = tip;
        loop {
            let blockhash = types::header_to_blockhash(&cur_hash);
            let path = self.config.get_blob_filepath(&blockhash);
            if path.exists() {
                let block = self.read_block(&blockhash).unwrap().decode().unwrap();
                let header = block.header();
                self.add_loose_to_index(&header);
                cur_hash = header.previous_header();
                continue;
            }
            break;
        }
    }

    /// Returns an iterator over blocks in the given block range.
    ///
    /// The range is given inclusively. The blocks are iterated in order from
    /// earlier to later.
    pub fn range(&self, from: BlockHash, to: BlockHash) -> Result<iter::Range> {
        iter::range_iter(self, from, to)
    }

    /// Returns an iterator over blocks in reverse from the given header hash.
    pub fn reverse_from(&self, hh: HeaderHash) -> Result<iter::ReverseIter> {
        iter::reverse_iter(self, hh)
    }

    pub fn block_location(&self, hash: &BlockHash) -> Result<BlockLocation> {
        for (packref, lookup) in self.lookups.iter() {
            let (start, nb) = lookup.fanout.get_indexer_by_hash(hash);
            match nb {
                indexfile::FanoutNb(0) => {}
                _ => {
                    if lookup.bloom.search(hash) {
                        let idx_filepath = self.config.get_index_filepath(packref);
                        let mut idx_file =
                            try_open!(indexfile::Reader::init, &idx_filepath, "index file");
                        let sr = idx_file.search(&lookup.params, hash, start, nb);
                        if let Some(iloc) = sr {
                            return Ok(BlockLocation::Packed(packref.clone(), iloc));
                        }
                    }
                }
            }
        }
        if blob::exist(self, hash) {
            return Ok(BlockLocation::Loose(hash.clone()));
        }
        Err(Error::BlockNotFound(hash.clone()))
    }

    pub fn block_location_by_height(&self, height: u64) -> Result<BlockLocation> {
        match self.get_from_loose_index(ChainDifficulty::from(height)) {
            Some((diff,date,hash)) => {
                debug!("Search in loose index by height {:?} returned ({:?}, {:?}, {:?})",
                       height, diff, date, hex::encode(&hash));
                Ok(BlockLocation::Loose(hash))
            },
            None => {
                let mut h = height;
                for (packref, lookup) in self.lookups.iter() {
                    let total = u32::from(lookup.fanout.get_total());
                    if h < (total as u64) {
                        let epoch_id = lookup.params.ordinal as u64;
                        let ofs = epoch::epoch_read_block_offset(&self.config, epoch_id, h as u32)?;
                        return Ok(BlockLocation::Offset(packref.clone(), ofs));
                    }
                    h -= total as u64;
                }
                Err(Error::BlockHeightNotFound(height))
            }
        }
    }

    pub fn block_exists(&self, hash: &BlockHash) -> Result<bool> {
        match self.block_location(hash) {
            Ok(_) => Ok(true),
            Err(Error::BlockNotFound(_)) => Ok(false),
            Err(err) => Err(err),
        }
    }

    pub fn read_block_at(&self, loc: &BlockLocation) -> Result<RawBlock> {
        match loc {
            BlockLocation::Loose(hash) => blob::read(self, hash),
            BlockLocation::Packed(ref packref, ref iofs) => match self.lookups.get(packref) {
                None => {
                    unreachable!();
                }
                Some(lookup) => {
                    let idx_filepath = self.config.get_index_filepath(packref);
                    let mut idx_file =
                        try_open!(indexfile::ReaderNoLookup::init, &idx_filepath, "index file");
                    let pack_offset = idx_file.resolve_index_offset(lookup, *iofs);
                    self.read_block_at(&BlockLocation::Offset(*packref, pack_offset))
                }
            },
            BlockLocation::Offset(ref packref, ref offset) => {
                let pack_filepath = self.config.get_pack_filepath(packref);
                let mut pack_file =
                    try_open!(packfile::Seeker::init, &pack_filepath, "pack file");
                let rblk = pack_file
                    .block_at_offset(*offset)
                    .and_then(|x| Ok(RawBlock(x)))?;
                Ok(rblk)
            }
        }
    }

    pub fn read_block(&self, hash: &BlockHash) -> Result<RawBlock> {
        let loc = self.block_location(hash)?;
        self.read_block_at(&loc)
    }

    pub fn get_block_from_tag(&self, tag: &str) -> Result<Block> {
        match tag::read_hash(&self, &tag) {
            None => Err(Error::NoSuchTag),
            Some(hash) => Ok(self.read_block(&hash.as_hash_bytes())?.decode()?),
        }
    }

    pub fn add_lookup(&mut self, packhash: PackHash, lookup: indexfile::Lookup) {
        self.lookups.insert(packhash, lookup);
    }

    pub fn add_loose_to_index(&mut self, header: &BlockHeaderView) {
        self.loose_idx.insert(0,(
            header.difficulty(),
            header.blockdate(),
            types::header_to_blockhash(&header.compute_hash()),
        ));
    }

    pub fn drop_loose_index_before(&mut self, diff: ChainDifficulty) {
        if self.loose_idx.is_empty() {
            return;
        }
        let drop_diff = u64::from(diff);
        let tip_diff = u64::from(self.loose_idx[0].0);
        let drop_after = (tip_diff - drop_diff) as usize;
        if self.loose_idx.len() > drop_after {
            self.loose_idx = self.loose_idx[0..drop_after].to_vec();
        }
    }

    pub fn get_from_loose_index(
        &self,
        diff: ChainDifficulty
    ) -> Option<(ChainDifficulty, BlockDate, BlockHash)> {
        if !self.loose_idx.is_empty() {
            let find_diff = u64::from(diff);
            let tip_diff = u64::from(self.loose_idx[0].0);
            let idx = (tip_diff - find_diff) as usize;
            if self.loose_idx.len() > idx {
                return Some(self.loose_idx[idx]);
            }
        }
        return None;
    }
}

fn tmpfile_create_type(storage: &Storage, filetype: StorageFileType) -> TmpFile {
    TmpFile::create(storage.config.get_filetype_dir(filetype)).unwrap()
}

pub mod blob {
    use super::Result;
    use cardano::block::RawBlock;
    use magic;
    use std::fs;
    use std::io::{Read, Write};

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
            Ok(()) => {}
            Err(_) => {}
        }
    }
}

#[derive(Clone, Debug)]
pub enum BlockLocation {
    Packed(PackHash, indexfile::IndexOffset),
    Offset(PackHash, serialize::Offset),
    Loose(BlockHash),
}

enum ReverseSearch {
    Continue,
    Found,
    Abort,
}

fn previous_block(storage: &Storage, block: &Block) -> Block {
    let prev_hash = block.header().previous_header();
    let blk = blob::read(&storage, &header_to_blockhash(&prev_hash))
        .unwrap()
        .decode()
        .unwrap();
    blk
}

fn block_reverse_search_from_tip<F>(
    storage: &Storage,
    first_block: &Block,
    find: F,
) -> Result<Option<Block>>
where
    F: Fn(&Block) -> Result<ReverseSearch>,
{
    let mut current_blk = first_block.clone();
    loop {
        match find(&current_blk)? {
            ReverseSearch::Continue => {
                let blk = previous_block(&storage, &current_blk);
                current_blk = blk;
            }
            ReverseSearch::Found => return Ok(Some(current_blk)),
            ReverseSearch::Abort => return Ok(None),
        };
    }
}

pub fn resolve_date_to_blockhash(
    storage: &Storage,
    tip: &HeaderHash,
    date: &BlockDate,
) -> Result<Option<BlockHash>> {
    let epoch = date.get_epochid();
    match epoch::epoch_open_packref(&storage.config, epoch) {
        Ok(mut handle) => {
            let slotid = match date {
                BlockDate::Boundary(_) => 0,
                BlockDate::Normal(sid) => sid.slotid,
            };
            let r = handle.getref_at_index(slotid as u32)?;
            Ok(r)
        }
        Err(_) => match storage.read_block(tip.as_hash_bytes()) {
            Err(Error::BlockNotFound(_)) => Ok(None),
            Err(err) => Err(err),
            Ok(rblk) => {
                let blk = rblk.decode()?;
                let found = block_reverse_search_from_tip(storage, &blk, |x| {
                    match x.header().blockdate().cmp(date) {
                        Ordering::Equal => Ok(ReverseSearch::Found),
                        Ordering::Greater => Ok(ReverseSearch::Continue),
                        Ordering::Less => Ok(ReverseSearch::Abort),
                    }
                })?;
                Ok(found.map(|x| header_to_blockhash(&x.header().compute_hash())))
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

    let block_hashes: Vec<BlockHash> = if let Some((from, to)) = params.range {
        storage.range(from, to).unwrap().iter().cloned().collect()
    } else {
        storage.config.list_blob(params.limit_nb_blobs)
    };
    for bh in block_hashes {
        let blob = blob::read_raw(storage, &bh).unwrap();
        writer.append(&bh, &blob[..]).unwrap();
        blob_packed.push(bh);
        match params.limit_size {
            None => {}
            Some(sz) => {
                if writer.pos() >= sz {
                    break;
                }
            }
        }
    }

    let (packhash, index) = pack::packwriter_finalize(&storage.config, writer);

    let (lookup, tmpfile) = pack::create_index(storage, &index, 0);
    tmpfile
        .render_permanent(&storage.config.get_index_filepath(&packhash))
        .unwrap();

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
    let mut packhash = [0; HASH_SIZE];
    packhash[..].clone_from_slice(packhash_vec.as_slice());
    let mut pack = packreader_init(&storage.config, &packhash);

    let mut current_state = None;

    while let Some(raw_block) = packreader_block_next(&mut pack)? {
        let block = raw_block.decode()?;
        let hdr = block.header();
        let hash = hdr.compute_hash();
        let date = hdr.blockdate();

        // either we have seen genesis yet or not
        match current_state {
            None => {
                if !hdr.is_boundary_block() {
                    return Err(Error::EpochExpectingBoundary);
                }
                current_state = Some((hdr.blockdate().get_epochid(), 0, hdr.compute_hash()));
                rp.append_hash(hash.into());
            }
            Some((current_epoch, expected_slotid, current_prevhash)) => match date.clone() {
                cardano::block::BlockDate::Boundary(_) => {
                    return Err(Error::RefPackUnexpectedBoundary(expected_slotid));
                }
                cardano::block::BlockDate::Normal(ref slotid) => {
                    if slotid.epoch != current_epoch {
                        return Err(Error::EpochError(current_epoch, slotid.epoch));
                    }
                    if slotid.slotid < expected_slotid {
                        return Err(Error::EpochSlotRewind(current_epoch, slotid.slotid));
                    }
                    let prevhash = hdr.previous_header();
                    if prevhash != current_prevhash {
                        return Err(Error::EpochChainInvalid(date, prevhash, current_prevhash));
                    }

                    let mut current_slotid = expected_slotid;

                    while current_slotid < slotid.slotid {
                        rp.append_missing_hash();
                        current_slotid += 1;
                    }
                    rp.append_hash(hash.clone().into());
                    current_state = Some((current_epoch, current_slotid, hash));
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
fn epoch_integrity_check(
    storage: &Storage,
    epochid: EpochId,
    last_known_hash: HeaderHash,
) -> Result<HeaderHash> {
    let packhash_vec = tag::read(storage, &format!("EPOCH_{}", epochid)).expect("EPOCH not found");
    let mut packhash = [0; HASH_SIZE];
    packhash[..].clone_from_slice(packhash_vec.as_slice());
    let mut pack = packreader_init(&storage.config, &packhash);

    let mut current_state = None;

    while let Some(raw_block) = packreader_block_next(&mut pack)? {
        let block = raw_block.decode()?;
        let hdr = block.header();
        let hash = hdr.compute_hash();
        let prevhash = hdr.previous_header();
        let date = hdr.blockdate();

        // either we have seen genesis yet or not
        match current_state {
            None => {
                if !hdr.is_boundary_block() {
                    return Err(Error::EpochExpectingBoundary);
                }
                if last_known_hash != prevhash {
                    return Err(Error::EpochChainInvalid(date, last_known_hash, prevhash));
                }
                current_state = Some((date.get_epochid(), 0, hdr.compute_hash()));
            }
            Some((current_epoch, expected_slotid, current_prevhash)) => match date.clone() {
                cardano::block::BlockDate::Boundary(_) => {
                    return Err(Error::RefPackUnexpectedBoundary(expected_slotid));
                }
                cardano::block::BlockDate::Normal(slotid) => {
                    if slotid.epoch != current_epoch {
                        return Err(Error::EpochError(current_epoch, slotid.epoch));
                    }
                    if slotid.slotid < expected_slotid {
                        return Err(Error::EpochSlotRewind(current_epoch, slotid.slotid));
                    }
                    if prevhash != current_prevhash {
                        return Err(Error::EpochChainInvalid(
                            date.clone(),
                            prevhash,
                            current_prevhash,
                        ));
                    }

                    let mut current_slotid = expected_slotid;

                    while current_slotid < slotid.slotid {
                        current_slotid += 1;
                    }
                    current_state = Some((current_epoch, current_slotid, hash));
                }
            },
        }
    }
    match current_state {
        None => panic!("test"),
        Some((_, _, prevhash)) => return Ok(prevhash),
    }
}
