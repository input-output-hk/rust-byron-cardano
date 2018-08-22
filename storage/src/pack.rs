
use std::io;
use std::io::{Write};
use std::fs;
use cryptoxide::blake2b;
use cryptoxide::digest::Digest;
use types::HASH_SIZE;
use utils::tmpfile::{TmpFile};
use utils::serialize::{write_size, Offset, Size, SIZE_SIZE};
use cardano;

use containers::packfile;
use containers::indexfile;

pub fn create_index(storage: &super::Storage, index: &indexfile::Index) -> (indexfile::Lookup, super::TmpFile) {
    let mut tmpfile = super::tmpfile_create_type(storage, super::StorageFileType::Index);
    let lookup = index.write_to_tmpfile(&mut tmpfile).unwrap();
    (lookup, tmpfile)
}

pub fn open_index(storage_config: &super::StorageConfig, pack: &super::PackHash) -> fs::File {
    fs::File::open(storage_config.get_index_filepath(pack)).unwrap()
}

pub fn dump_index(storage_config: &super::StorageConfig, pack: &super::PackHash) -> io::Result<(indexfile::Lookup, Vec<super::BlockHash>)> {
    let mut file = open_index(storage_config, pack);
    indexfile::dump_file(&mut file)
}

pub fn read_index_fanout(storage_config: &super::StorageConfig, pack: &super::PackHash) -> io::Result<indexfile::Lookup> {
    let mut file = open_index(storage_config, pack);
    indexfile::Lookup::read_from_file(&mut file)
}

pub fn index_get_header(file: &fs::File) -> io::Result<indexfile::Lookup> {
    indexfile::Lookup::read_from_file(file)
}

// A Writer for a specific pack that accumulate some numbers for reportings,
// index, blobs_hashes for index creation (in finalize)
pub struct PackWriter {
    tmpfile: TmpFile,
    index: indexfile::Index,
    pub nb_blobs: u32,
    pub pos: Offset, // offset in bytes of the current position (double as the current size of the pack)
    hash_context: blake2b::Blake2b, // hash of all the content of blocks without length or padding
    storage_config: super::StorageConfig,
}

impl PackWriter {
    pub fn init(cfg: &super::StorageConfig) -> Self {
        let tmpfile = TmpFile::create(cfg.get_filetype_dir(super::StorageFileType::Pack)).unwrap();
        let idx = indexfile::Index::new();
        let ctxt = blake2b::Blake2b::new(32);
        PackWriter
            { tmpfile: tmpfile, index: idx, pos: 0, nb_blobs: 0, storage_config: cfg.clone(), hash_context: ctxt }
    }

    pub fn get_current_size(&self) -> u64 {
        self.pos
    }

    pub fn get_current_number_of_blobs(&self) -> u32 {
        self.nb_blobs
    }

    pub fn append_raw(&mut self, blockhash: &super::BlockHash, block: &[u8]) {
        let len = block.len() as Size;
        let mut sz_buf = [0u8;SIZE_SIZE];
        write_size(&mut sz_buf, len);
        self.tmpfile.write_all(&sz_buf[..]).unwrap();
        self.tmpfile.write_all(block).unwrap();
        self.hash_context.input(block);

        let pad = [0u8;SIZE_SIZE-1];
        let pad_bytes = if (len % 4 as u32) != 0 {
                            let pad_sz = 4 - len % 4;
                            self.tmpfile.write_all(&pad[0..pad_sz as usize]).unwrap();
                            pad_sz
                        } else { 0 };
        self.index.append(blockhash, self.pos);
        self.pos += 4 + len as u64 + pad_bytes as u64;
        self.nb_blobs += 1;
    }

    pub fn append(&mut self, blockhash: &super::BlockHash, block: &[u8]) {
        self.append_raw(blockhash, block)
    }

    pub fn finalize(&mut self) -> (super::PackHash, indexfile::Index) {
        let mut packhash : super::PackHash = [0u8;HASH_SIZE];
        self.hash_context.result(&mut packhash);
        let path = self.storage_config.get_pack_filepath(&packhash);
        self.tmpfile.render_permanent(&path).unwrap();
        (packhash, self.index.clone())
    }
}

pub fn packreader_init(cfg: &super::StorageConfig, packhash: &super::PackHash) -> packfile::Reader<fs::File> {
    packfile::Reader::init(cfg.get_pack_filepath(packhash)).unwrap()
}

pub fn packreader_block_next(reader: &mut packfile::Reader<fs::File>) -> Option<cardano::block::RawBlock> {
    reader.get_next().and_then(|x| Some(cardano::block::RawBlock(x)))
}