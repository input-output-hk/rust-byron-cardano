//! Indexfile format
//!
//! An index file is:
//!
//! MAGIC (8 Bytes)
//! BLOOM SIZE (4 bytes BE)
//! 0-PADDING (4 bytes)
//! FANOUT (256*4 bytes)
//! BLOOM FILTER (BLOOM_SIZE bytes)
//! BLOCK HASHES present in this pack ordered lexigraphically (#ENTRIES * 32 bytes)
//! OFFSET of BLOCK in the same order as BLOCK_HASHES (#ENTRIES * 8 bytes)
//!
//! The fanout is a cumulative numbers of things stored, ordered by their hash and
//! group in 256 buckets (first byte of the hash). This give a very efficient
//! way to "zoom" on the BLOCK HASHES, at it allows to windows only the hash that
//! start with a specific byte. This improve efficiency when searching inside a pack.
//!
//! The bloom filter is an help to the overall pack search, it allows to
//! efficiently query whether or not a hash is likely to be here or not. By the
//! nature of a bloom filter, it can only answer with certainty whether it
//! is present in this pack, there will be false positive in search.
//!

use hash::{BlockHash, HASH_SIZE};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::iter::repeat;
use std::path::Path;
use utils::bloom;
use utils::error::Result;
use utils::magic;
use utils::serialize::{
    read_offset, read_size, write_offset, write_size, Offset, OFF_SIZE, SIZE_SIZE,
};
use utils::tmpfile::TmpFile;

const FILE_TYPE: magic::FileType = 0x494e4458; // = INDX
const VERSION: magic::Version = 1;

const FANOUT_ELEMENTS: usize = 256;
const FANOUT_SIZE: usize = FANOUT_ELEMENTS * SIZE_SIZE;

const HEADER_SIZE: usize = BLOOM_OFFSET - magic::HEADER_SIZE;

const FANOUT_OFFSET: usize = magic::HEADER_SIZE + 8;
const BLOOM_OFFSET: usize = FANOUT_OFFSET + FANOUT_SIZE;

// calculate the file offset from where the hashes are stored
fn offset_hashes(bloom_size: u32) -> u64 {
    magic::HEADER_SIZE as u64 + 8 + FANOUT_SIZE as u64 + bloom_size as u64
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

pub struct Fanout([u32; FANOUT_ELEMENTS]);
pub struct FanoutStart(u32);
pub struct FanoutNb(pub u32);
pub struct FanoutTotal(u32);

impl Fanout {
    pub fn get_indexer_by_hash(&self, hash: &BlockHash) -> (FanoutStart, FanoutNb) {
        self.get_indexer_by_hier(hash[0])
    }

    pub fn get_indexer_by_hier(&self, hier: u8) -> (FanoutStart, FanoutNb) {
        match hier as usize {
            0 => (FanoutStart(0), FanoutNb(self.0[0])),
            c => {
                let start = self.0[c - 1];
                (FanoutStart(start), FanoutNb(self.0[c] - start))
            }
        }
    }
    pub fn get_total(&self) -> FanoutTotal {
        FanoutTotal(self.0[255])
    }
}
impl From<FanoutTotal> for u32 {
    fn from(ft: FanoutTotal) -> Self {
        ft.0
    }
}

pub struct Bloom(Vec<u8>);

impl Bloom {
    pub fn search(&self, blk: &BlockHash) -> bool {
        bloom::is_set(&self.0[..], blk)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
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

#[derive(Clone)]
pub struct Index {
    pub hashes: Vec<BlockHash>,
    pub offsets: Vec<Offset>,
}

impl Index {
    pub fn new() -> Self {
        Index {
            hashes: Vec::new(),
            offsets: Vec::new(),
        }
    }

    pub fn append(&mut self, hash: &BlockHash, offset: Offset) {
        self.hashes.push(hash.clone());
        self.offsets.push(offset);
    }

    pub fn write_to_tmpfile(&self, tmpfile: &mut TmpFile) -> Result<Lookup> {
        magic::write_header(tmpfile, FILE_TYPE, VERSION)?;

        let mut hdr_buf = [0u8; HEADER_SIZE];

        let entries = self.hashes.len();

        assert!(entries == self.offsets.len());

        let bloom_size = default_bloom_size(entries);
        let params = Params {
            bloom_size: bloom_size,
        };

        write_size(&mut hdr_buf[0..4], bloom_size);
        write_size(&mut hdr_buf[4..8], 0);

        // write fanout to hdr_buf
        let fanout = {
            let mut fanout_abs = [0u32; FANOUT_ELEMENTS];
            for &hash in self.hashes.iter() {
                let ofs = hash[0] as usize;
                fanout_abs[ofs] = fanout_abs[ofs] + 1;
            }
            let mut fanout_sum = 0;
            let mut fanout_incr = [0u32; FANOUT_ELEMENTS];
            for i in 0..FANOUT_ELEMENTS {
                fanout_sum += fanout_abs[i];
                fanout_incr[i] = fanout_sum;
            }

            for i in 0..FANOUT_ELEMENTS {
                let ofs = FANOUT_OFFSET + i * SIZE_SIZE - magic::HEADER_SIZE;
                write_size(&mut hdr_buf[ofs..ofs + SIZE_SIZE], fanout_incr[i]);
            }
            Fanout(fanout_incr)
        };
        tmpfile.write_all(&hdr_buf)?;

        let mut bloom: Vec<u8> = repeat(0).take(bloom_size as usize).collect();
        for hash in self.hashes.iter() {
            bloom::set(&mut bloom[..], hash);
        }

        tmpfile.write_all(&bloom[..])?;

        let mut sorted = Vec::with_capacity(entries);
        for i in 0..entries {
            sorted.push((self.hashes[i], self.offsets[i]));
        }
        sorted.sort_by(|a, b| a.0.cmp(&b.0));

        for &(hash, _) in sorted.iter() {
            tmpfile.write_all(&hash[..])?;
        }

        write_offsets_to_file(tmpfile, sorted.iter().map(|(_, b)| b))?;

        Ok(Lookup {
            params: params,
            fanout: fanout,
            bloom: Bloom(bloom),
        })
    }
}

impl Lookup {
    pub fn read_from_file(file: &mut fs::File) -> Result<Self> {
        magic::check_header(file, FILE_TYPE, VERSION, VERSION)?;
        let mut hdr_buf = [0u8; HEADER_SIZE];

        file.read_exact(&mut hdr_buf)?;
        let bloom_size = read_size(&hdr_buf[0..4]);

        let mut fanout = [0u32; FANOUT_ELEMENTS];
        for i in 0..FANOUT_ELEMENTS {
            let ofs = FANOUT_OFFSET + i * SIZE_SIZE - magic::HEADER_SIZE;
            fanout[i] = read_size(&hdr_buf[ofs..ofs + SIZE_SIZE])
        }
        let mut bloom: Vec<u8> = repeat(0).take(bloom_size as usize).collect();

        file.read_exact(&mut bloom[..])?;

        Ok(Lookup {
            params: Params {
                bloom_size: bloom_size,
            },
            fanout: Fanout(fanout),
            bloom: Bloom(bloom),
        })
    }
}

pub fn write_offsets_to_file<'a, I: Iterator<Item = &'a Offset>>(
    tmpfile: &mut TmpFile,
    offsets: I,
) -> Result<()> {
    for ofs in offsets {
        let mut buf = [0u8; OFF_SIZE];
        write_offset(&mut buf, *ofs);
        tmpfile.write_all(&buf[..])?;
    }
    Ok(())
}

fn file_read_offset(mut file: &fs::File) -> Offset {
    let mut buf = [0u8; OFF_SIZE];
    file.read_exact(&mut buf).unwrap();
    read_offset(&buf)
}

pub fn file_read_offset_at(mut file: &fs::File, ofs: u64) -> Offset {
    file.seek(SeekFrom::Start(ofs)).unwrap();
    file_read_offset(file)
}

fn file_read_hash(mut file: &fs::File) -> BlockHash {
    let mut buf = [0u8; HASH_SIZE];
    file.read_exact(&mut buf).unwrap();
    buf
}

pub fn dump_file(file: &mut fs::File) -> Result<(Lookup, Vec<BlockHash>)> {
    let lookup = Lookup::read_from_file(file)?;

    let mut v = Vec::new();
    let FanoutTotal(total) = lookup.fanout.get_total();

    file.seek(SeekFrom::Start(HEADER_SIZE as u64)).unwrap();
    for _ in 0..total {
        let h = file_read_hash(file);
        v.push(h);
    }
    Ok((lookup, v))
}

pub struct ReaderNoLookup<R> {
    handle: R,
}

impl ReaderNoLookup<fs::File> {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        // TODO : just read the magic
        let _ = Lookup::read_from_file(&mut file)?;
        Ok(ReaderNoLookup { handle: file })
    }
    pub fn resolve_index_offset(&mut self, lookup: &Lookup, index_offset: IndexOffset) -> Offset {
        let FanoutTotal(total) = lookup.fanout.get_total();
        let ofs_base = offset_offsets(lookup.params.bloom_size, total);
        let ofs = ofs_base + OFF_SIZE as u64 * index_offset as u64;
        file_read_offset_at(&mut self.handle, ofs)
    }
}

pub struct Reader<R> {
    lookup: Lookup,
    handle: R,
}

impl Reader<fs::File> {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        let lookup = Lookup::read_from_file(&mut file)?;
        Ok(Reader {
            lookup: lookup,
            handle: file,
        })
    }

    // conduct a search in the index file, returning the offset index of a found element
    //
    // TODO switch to bilinear search with n > something
    pub fn search(
        &mut self,
        params: &Params,
        blk: &BlockHash,
        start_elements: FanoutStart,
        hier_elements: FanoutNb,
    ) -> Option<IndexOffset> {
        let hsz = offset_hashes(params.bloom_size);
        match hier_elements.0 {
            0 => None,
            1 => {
                let ofs_element = start_elements.0;
                let ofs = ofs_element as u64 * HASH_SIZE as u64;
                self.handle.seek(SeekFrom::Start(hsz + ofs)).unwrap();
                let hash = file_read_hash(&mut self.handle);
                if &hash == blk {
                    Some(ofs_element)
                } else {
                    None
                }
            }
            2 => {
                let ofs_element = start_elements.0;
                let ofs = ofs_element as u64 * HASH_SIZE as u64;
                self.handle.seek(SeekFrom::Start(hsz + ofs)).unwrap();
                let hash = file_read_hash(&mut self.handle);
                let hash2 = file_read_hash(&mut self.handle);
                if &hash == blk {
                    Some(ofs_element)
                } else if &hash2 == blk {
                    Some(ofs_element + 1)
                } else {
                    None
                }
            }
            n => {
                let start = start_elements.0;
                let end = start_elements.0 + n;
                let mut ofs_element = start;
                let ofs = ofs_element as u64 * HASH_SIZE as u64;
                self.handle.seek(SeekFrom::Start(hsz + ofs)).unwrap();
                while ofs_element < end {
                    let hash = file_read_hash(&mut self.handle);
                    if &hash == blk {
                        return Some(ofs_element);
                    }
                    ofs_element += 1
                }
                None
            }
        }
    }

    pub fn resolve_index_offset(&mut self, index_offset: IndexOffset) -> Offset {
        let FanoutTotal(total) = self.lookup.fanout.get_total();
        let ofs_base = offset_offsets(self.lookup.params.bloom_size, total);
        let ofs = ofs_base + OFF_SIZE as u64 * index_offset as u64;
        self.handle.seek(SeekFrom::Start(ofs)).unwrap();
        file_read_offset(&mut self.handle)
    }
}
