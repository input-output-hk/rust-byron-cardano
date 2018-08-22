// a pack file is a collection of blobs, prefixed by their 32 bits size in BE:
//
// SIZE (4 bytes BE)
// DATA (SIZE bytes)
// OPTIONAL ALIGNMENT? (of 0 to 3 bytes depending on SIZE)
//
// An index file is:
//
// MAGIC (8 Bytes)
// BLOOM SIZE (4 bytes BE)
// 0-PADDING (4 bytes)
// FANOUT (256*4 bytes)
// BLOOM FILTER (BLOOM_SIZE bytes)
// BLOCK HASHES present in this pack ordered lexigraphically (#ENTRIES * 32 bytes)
// OFFSET of BLOCK in the same order as BLOCK_HASHES (#ENTRIES * 8 bytes)
//
// The fanout is a cumulative numbers of things stored, ordered by their hash and
// group in 256 buckets (first byte of the hash). This give a very efficient
// way to "zoom" on the BLOCK HASHES, at it allows to windows only the hash that
// start with a specific byte. This improve efficiency when searching inside a pack.
//
// The bloom filter is an help to the overall pack search, it allows to
// efficiently query whether or not a hash is likely to be here or not. By the
// nature of a bloom filter, it can only answer with certainty whether it
// is present in this pack, there will be false positive in search.
//

use std::iter::repeat;
use std::io::SeekFrom;
use std::io;
use std::io::{Write,Read,Seek,ErrorKind};
use std::fs;
use cryptoxide::blake2b;
use cryptoxide::digest::Digest;
use types::HASH_SIZE;
use utils::bloom;
use utils::tmpfile::{TmpFile};
use utils::serialize::{read_offset, read_size, write_offset, write_size, offset_align4, Offset, Size, SIZE_SIZE, OFF_SIZE};
use types::BlockHash;
use cardano;

use containers::packfile;

const MAGIC : &[u8] = b"ADAPACK1";
const MAGIC_SIZE : usize = 8;

const FANOUT_ELEMENTS : usize = 256;
const FANOUT_SIZE : usize = FANOUT_ELEMENTS*SIZE_SIZE;

const HEADER_SIZE : usize = BLOOM_OFFSET as usize;

const FANOUT_OFFSET : usize = MAGIC_SIZE + 8;
const BLOOM_OFFSET : usize = FANOUT_OFFSET + FANOUT_SIZE;

// calculate the file offset from where the hashes are stored
fn offset_hashes(bloom_size: u32) -> u64 {
    8 + 8 + FANOUT_SIZE as u64 + bloom_size as u64
}

// calculate the file offset from where the offsets are stored
fn offset_offsets(bloom_size: u32, number_hashes: u32) -> u64 {
    offset_hashes(bloom_size) + HASH_SIZE as u64 * number_hashes as u64
}

pub type IndexOffset = u32;

// The parameters associated with the index file.
// * the bloom filter size in bytes
pub struct Params {
    pub bloom_size: u32,
}

pub struct Lookup {
    pub params: Params,
    pub fanout: Fanout,
    pub bloom: Bloom,
}

pub struct Fanout([u32;FANOUT_ELEMENTS]);
pub struct FanoutStart(u32);
pub struct FanoutNb(pub u32);
pub struct FanoutTotal(u32);

impl Fanout {
    pub fn get_indexer_by_hash(&self, hash: &super::BlockHash) -> (FanoutStart, FanoutNb) {
        self.get_indexer_by_hier(hash[0])
    }

    pub fn get_indexer_by_hier(&self, hier: u8) -> (FanoutStart, FanoutNb) {
        match hier as usize {
            0 => (FanoutStart(0), FanoutNb(self.0[0])),
            c => {
                let start = self.0[c-1];
                (FanoutStart(start), FanoutNb(self.0[c] - start))
            },
        }
    }
    pub fn get_total(&self) -> FanoutTotal {
        FanoutTotal(self.0[255])
    }
}

pub struct Bloom(Vec<u8>);

impl Bloom {
    pub fn search(&self, blk: &BlockHash) -> bool {
        bloom::is_set(&self.0[..], blk)
    }

    pub fn len(&self) -> usize { self.0.len() }
}

fn file_read_offset(mut file: &fs::File) -> Offset {
    let mut buf = [0u8;OFF_SIZE];
    file.read_exact(&mut buf).unwrap();
    read_offset(&buf)
}

fn file_read_hash(mut file: &fs::File) -> super::BlockHash {
    let mut buf = [0u8;HASH_SIZE];
    file.read_exact(&mut buf).unwrap();
    buf
}

// the default size (in bytes) of the bloom filter related to the number of
// expected entries in the files.
pub fn default_bloom_size(entries: usize) -> u32 {
    if entries < 0x1000 {
        4096
    } else if entries < 0x5000 {
        8192
    } else if entries < 0x22000 {
        16384
    } else {
        32768
    }
}

pub fn create_index(storage: &super::Storage, index: &Index) -> (Lookup, super::TmpFile) {
    let mut tmpfile = super::tmpfile_create_type(storage, super::StorageFileType::Index);
    let mut hdr_buf = [0u8;HEADER_SIZE];

    let entries = index.hashes.len();

    assert!(entries == index.offsets.len());

    let bloom_size = default_bloom_size(entries);
    let params = Params { bloom_size: bloom_size };

    hdr_buf[0..8].clone_from_slice(&MAGIC[..]);
    write_size(&mut hdr_buf[8..12], bloom_size as u32);
    write_size(&mut hdr_buf[12..16], 0);

    // write fanout to hdr_buf
    let fanout = {
        let mut fanout_abs = [0u32;FANOUT_ELEMENTS];
        for &hash in index.hashes.iter() {
            let ofs = hash[0] as usize;
            fanout_abs[ofs] = fanout_abs[ofs]+1;
        }
        let mut fanout_sum = 0;
        let mut fanout_incr = [0u32;FANOUT_ELEMENTS];
        for i in 0..FANOUT_ELEMENTS {
            fanout_sum += fanout_abs[i];
            fanout_incr[i] = fanout_sum;
        }

        for i in 0..FANOUT_ELEMENTS {
            let ofs = FANOUT_OFFSET + i * SIZE_SIZE; /* start at 16, because 0..8 is the magic, followed by size, and padding */
            write_size(&mut hdr_buf[ofs..ofs+SIZE_SIZE], fanout_incr[i]);
        }
        Fanout(fanout_incr)
    };
    tmpfile.write_all(&hdr_buf).unwrap();

    let mut bloom : Vec<u8> = repeat(0).take(bloom_size as usize).collect();
    for hash in index.hashes.iter() {
        bloom::set(&mut bloom[..], hash);
    }

    tmpfile.write_all(&bloom[..]).unwrap();

    let mut sorted = Vec::with_capacity(entries);
    for i in 0..entries {
        sorted.push((index.hashes[i], index.offsets[i]));
    }
    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    for &(hash,_) in sorted.iter() {
        tmpfile.write_all(&hash[..]).unwrap();
    }

    for &(_, ofs) in sorted.iter() {
        let mut buf = [0u8;OFF_SIZE];
        write_offset(&mut buf, ofs);
        tmpfile.write_all(&buf[..]).unwrap();
    }
    (Lookup { params: params, fanout: fanout, bloom: Bloom(bloom) }, tmpfile)
}

pub fn open_index(storage_config: &super::StorageConfig, pack: &super::PackHash) -> fs::File {
    fs::File::open(storage_config.get_index_filepath(pack)).unwrap()
}

pub fn dump_index(storage_config: &super::StorageConfig, pack: &super::PackHash) -> io::Result<(Lookup, Vec<super::BlockHash>)> {
    let mut file = open_index(storage_config, pack);
    let lookup = index_get_header(&mut file)?;

    let mut v = Vec::new();
    let FanoutTotal(total) = lookup.fanout.get_total();

    file.seek(SeekFrom::Start(HEADER_SIZE as u64)).unwrap();
    for _ in 0..total {
        let h = file_read_hash(&mut file);
        v.push(h);
    }
    Ok((lookup, v))
}

pub fn index_get_header(mut file: &fs::File) -> io::Result<Lookup> {
    let mut hdr_buf = [0u8;HEADER_SIZE];

    file.read_exact(&mut hdr_buf)?;
    if &hdr_buf[0..8] != MAGIC {
        return Err(io::Error::last_os_error());
    }
    let bloom_size = read_size(&hdr_buf[8..12]);

    let mut fanout = [0u32;FANOUT_ELEMENTS];
    for i in 0..FANOUT_ELEMENTS {
        let ofs = FANOUT_OFFSET+i*SIZE_SIZE;
        fanout[i] = read_size(&hdr_buf[ofs..ofs+SIZE_SIZE])
    }
    let mut bloom : Vec<u8> = repeat(0).take(bloom_size as usize).collect();

    file.read_exact(&mut bloom[..])?;

    Ok(Lookup {
        params: Params { bloom_size: bloom_size },
        fanout: Fanout(fanout),
        bloom: Bloom(bloom)
    })
}

pub fn read_index_fanout(storage_config: &super::StorageConfig, pack: &super::PackHash) -> io::Result<Lookup> {
    let mut file = open_index(storage_config, pack);
    index_get_header(&mut file)
}

// conduct a search in the index file, returning the offset index of a found element
//
// TODO switch to bilinear search with n > something
pub fn search_index(mut file: &fs::File, params: &Params, blk: &super::BlockHash, start_elements: FanoutStart, hier_elements: FanoutNb) -> Option<IndexOffset> {
    let hsz = offset_hashes(params.bloom_size);
    match hier_elements.0 {
        0 => None,
        1 => {
            let ofs_element = start_elements.0;
            let ofs = ofs_element as u64 * HASH_SIZE as u64;
            file.seek(SeekFrom::Start(hsz + ofs)).unwrap();
            let hash = file_read_hash(file);
            if &hash == blk { Some(ofs_element) } else { None }
        },
        2 => {
            let ofs_element = start_elements.0;
            let ofs = ofs_element as u64 * HASH_SIZE as u64;
            file.seek(SeekFrom::Start(hsz + ofs)).unwrap();
            let hash = file_read_hash(file);
            let hash2 = file_read_hash(file);
            if &hash == blk { Some(ofs_element) } else if &hash2 == blk { Some(ofs_element+1) } else { None }
        },
        n => {
            let start = start_elements.0;
            let end = start_elements.0 + n;
            let mut ofs_element = start;
            let ofs = ofs_element as u64 * HASH_SIZE as u64;
            file.seek(SeekFrom::Start(hsz + ofs)).unwrap();
            while ofs_element < end {
                let hash = file_read_hash(file);
                if &hash == blk {
                    return Some(ofs_element)
                }
                ofs_element += 1
            }
            None
        },
    }
}

pub fn resolve_index_offset(mut file: &fs::File, lookup: &Lookup, index_offset: IndexOffset) -> Offset {
    let FanoutTotal(total) = lookup.fanout.get_total();
    let ofs_base = offset_offsets(lookup.params.bloom_size, total);
    let ofs = ofs_base + OFF_SIZE as u64 * index_offset as u64;
    file.seek(SeekFrom::Start(ofs)).unwrap();
    file_read_offset(&mut file)
}

#[derive(Clone)]
pub struct Index {
    pub hashes: Vec<super::BlockHash>,
    pub offsets: Vec<Offset>,
}

impl Index {
    pub fn new() -> Self {
        Index { hashes: Vec::new(), offsets: Vec::new() }
    }

    pub fn append(&mut self, hash: &super::BlockHash, offset: Offset) {
        self.hashes.push(hash.clone());
        self.offsets.push(offset);
    }
}

// A Writer for a specific pack that accumulate some numbers for reportings,
// index, blobs_hashes for index creation (in finalize)
pub struct PackWriter {
    tmpfile: TmpFile,
    index: Index,
    pub nb_blobs: u32,
    pub pos: Offset, // offset in bytes of the current position (double as the current size of the pack)
    hash_context: blake2b::Blake2b, // hash of all the content of blocks without length or padding
    storage_config: super::StorageConfig,
}

impl PackWriter {
    pub fn init(cfg: &super::StorageConfig) -> Self {
        let tmpfile = TmpFile::create(cfg.get_filetype_dir(super::StorageFileType::Pack)).unwrap();
        let idx = Index::new();
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

    pub fn finalize(&mut self) -> (super::PackHash, Index) {
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