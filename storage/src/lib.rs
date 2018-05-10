extern crate rcw;
extern crate wallet_crypto;
extern crate blockchain;
extern crate rand;
extern crate flate2;

pub mod block;
pub mod types;
pub mod config;
pub mod pack;
pub mod tag;
pub mod refpack;
mod tmpfile;
mod bitmap;
mod bloom;
use std::{fs, io, result};

use std::collections::BTreeMap;

use types::*;
use config::*;
use tmpfile::*;

const USE_COMPRESSION : bool = true;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    BlockError(block::Error)
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IoError(e) }
}
impl From<block::Error> for Error {
    fn from(e: block::Error) -> Self { Error::BlockError(e) }
}

pub type Result<T> = result::Result<T, Error>;

pub struct Storage {
    config: StorageConfig,
    lookups: BTreeMap<PackHash, pack::Lookup>,
}

impl Storage {
    pub fn init(cfg: &StorageConfig) -> Result<Self> {
        let mut lookups = BTreeMap::new();

        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Blob))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Index))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Pack))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Tag))?;
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

    /// construct a range between the given hash
    pub fn range(&self, from: BlockHash, to: BlockHash) -> Result<block::Range> {
        block::Range::new(self, from, to).map_err(|err| Error::BlockError(err))
    }
}

fn tmpfile_create_type(storage: &Storage, filetype: StorageFileType) -> TmpFile {
    TmpFile::create(storage.config.get_filetype_dir(filetype)).unwrap()
}

pub mod blob {
    use std::fs;
    use std::io::{Write,Read};
    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use flate2::write::DeflateDecoder;

    use super::{Result, Error};

    pub fn write(storage: &super::Storage, hash: &super::BlockHash, block: &[u8]) -> Result<()> {
        let path = storage.config.get_blob_filepath(&hash);
        let mut tmp_file = super::tmpfile_create_type(storage, super::StorageFileType::Blob);
        if super::USE_COMPRESSION {
            let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
            e.write_all(block)?;
            let compressed_block = e.finish()?;
            tmp_file.write_all(&compressed_block[..])?;
        } else {
            tmp_file.write_all(block)?;
        }
        tmp_file.render_permanent(&path).map_err(|e| Error::IoError(e))
    }

    pub fn read_raw(storage: &super::Storage, hash: &super::BlockHash) -> Result<Vec<u8>> {
        let mut content = Vec::new();
        let path = storage.config.get_blob_filepath(&hash);

        let mut file = fs::File::open(path)?;
        file.read_to_end(&mut content)?;
        Ok(content)
    }

    pub fn read(storage: &super::Storage, hash: &super::BlockHash) -> Result<Vec<u8>> {
        let mut content = Vec::new();
        let path = storage.config.get_blob_filepath(&hash);

        let mut file = fs::File::open(path)?;
        file.read_to_end(&mut content)?;

        if super::USE_COMPRESSION {
            let mut writer = Vec::new();
            let mut deflater = DeflateDecoder::new(writer);
            deflater.write_all(&content[..])?;
            writer = deflater.finish()?;
            Ok(writer)
        } else {
            Ok(content)
        }
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
                let bloom_size = lookup.bloom.len();
                if lookup.bloom.search(hash) {
                    let idx_filepath = storage.config.get_index_filepath(packref);
                    let mut idx_file = fs::File::open(idx_filepath).unwrap();
                    match pack::search_index(&mut idx_file, bloom_size, hash, start, nb) {
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

pub fn block_read_location(storage: &Storage, loc: &BlockLocation, hash: &BlockHash) -> Option<Vec<u8>> {
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
                    Some(pack::read_block_at(&mut pack_file, pack_offset))
                }
            }
        }
    }
}

pub fn block_read(storage: &Storage, hash: &BlockHash) -> Option<Vec<u8>> {
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