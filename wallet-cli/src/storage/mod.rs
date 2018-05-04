pub mod types;
pub mod config;
pub mod pack;
pub mod tag;
mod tmpfile;
mod bitmap;
use std::{fs, io};

use std::collections::BTreeMap;

use rcw;

use self::types::*;
use self::config::*;
use self::tmpfile::*;

const USE_COMPRESSION : bool = true;

pub struct Storage {
    config: StorageConfig,
    lookups: BTreeMap<PackHash, pack::Lookup>,
}

impl Storage {
    pub fn init(cfg: &StorageConfig) -> io::Result<Self> {
        let mut lookups = BTreeMap::new();

        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Blob))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Index))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Pack))?;
        fs::create_dir_all(cfg.get_filetype_dir(StorageFileType::Tag))?;

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

    pub fn write(storage: &super::Storage, hash: &super::BlockHash, block: &[u8]) {
        let path = storage.config.get_blob_filepath(&hash);
        let mut tmp_file = super::tmpfile_create_type(storage, super::StorageFileType::Blob);
        if super::USE_COMPRESSION {
            let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
            e.write_all(block).unwrap();
            let compressed_block = e.finish().unwrap();
            tmp_file.write_all(&compressed_block[..]).unwrap();
        } else {
            tmp_file.write_all(block).unwrap();
        }
        tmp_file.render_permanent(&path).unwrap();

    }

    pub fn read_raw(storage: &super::Storage, hash: &super::BlockHash) -> Vec<u8> {
        let mut content = Vec::new();
        let path = storage.config.get_blob_filepath(&hash);

        let mut file = fs::File::open(path).unwrap();
        file.read_to_end(&mut content).unwrap();
        content
    }

    pub fn read(storage: &super::Storage, hash: &super::BlockHash) -> Vec<u8> {
        let mut content = Vec::new();
        let path = storage.config.get_blob_filepath(&hash);

        let mut file = fs::File::open(path).unwrap();
        file.read_to_end(&mut content).unwrap();

        if super::USE_COMPRESSION {
            let mut writer = Vec::new();
            let mut deflater = DeflateDecoder::new(writer);
            deflater.write_all(&content[..]).unwrap();
            writer = deflater.finish().unwrap();
            writer
        } else {
            content
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
                let idx_filepath = storage.config.get_index_filepath(packref);
                let mut idx_file = fs::File::open(idx_filepath).unwrap();
                match pack::search_index(&mut idx_file, hash, start, nb) {
                    None       => {},
                    Some(iloc) => return Some(BlockLocation::Packed(packref.clone(), iloc)),
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
        BlockLocation::Loose                 => Some(blob::read(storage, hash)),
        BlockLocation::Packed(packref, iofs) => {
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

// packing parameters
//
// optionally set the maximum number of blobs in this pack
// optionally set the maximum size in bytes of the pack file.
//            note that the limits is best effort, not strict.
pub struct PackParameters {
    pub limit_nb_blobs: Option<u32>,
    pub limit_size: Option<u64>,
    pub delete_blobs_after_pack: bool,
}

pub fn pack_blobs(storage: &mut Storage, params: &PackParameters) -> PackHash {
    let mut writer = pack::PackWriter::init(&storage.config);
    let block_hashes = storage.config.list_blob(params.limit_nb_blobs);
    let mut blob_packed = Vec::new();
    for bh in block_hashes.iter() {
        let blob = blob::read_raw(storage, bh);
        writer.append(bh, &blob[..]);
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