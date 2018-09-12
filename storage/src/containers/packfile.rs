/// packfile format
///
/// a pack file is a collection of blobs, prefixed by their 32 bits size in BE:
///
/// SIZE (4 bytes BE)
/// DATA (SIZE bytes)
/// OPTIONAL ALIGNMENT? (of 0 to 3 bytes depending on SIZE)
///

use std::io::{Read,Seek,SeekFrom, Write};
use std::io;
use std::fs;
use std::iter::repeat;
use std::path::Path;
use utils::serialize::{Offset, Size, SIZE_SIZE, read_size, write_size, offset_align4};
use utils::tmpfile::TmpFile;
use utils::error::Result;
use types::{PackHash, BlockHash, HASH_SIZE};
use cryptoxide::blake2b;
use cryptoxide::digest::Digest;
use containers::indexfile;
use magic;

const FILE_TYPE: magic::FileType = 0x5041434b; // = PACK
const VERSION: magic::Version = 1;

/// A Stream Reader that also computes the hash of the sum of all data read
pub struct Reader<R> {
    reader: R,
    pub pos: Offset,
    hash_context: blake2b::Blake2b, // hash of all the content of blocks without length or padding
}

/// A pack reader that can seek in a packfile
pub struct Seeker<R> {
    handle: R,
}

impl Reader<fs::File> {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        magic::check_header(&mut file, FILE_TYPE, VERSION, VERSION)?;
        Ok(Reader::from(file))
    }
}

impl Seeker<fs::File> {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        magic::check_header(&mut file, FILE_TYPE, VERSION, VERSION)?;
        Ok(Seeker::from(file))
    }
}

impl<R> From<R> for Reader<R> {
    fn from(reader: R) -> Self {
        let ctxt = blake2b::Blake2b::new(HASH_SIZE);
        Reader { reader, pos: 0, hash_context: ctxt }
    }
}

impl<R: Seek> From<R> for Seeker<R> {
    fn from(handle: R) -> Self {
        Seeker { handle }
    }
}

// a block in a pack file is:
// * a 32 bit size in big endian
// * data of the size above
// * 0 to 3 bytes of 0-alignment to make sure the next block is aligned
pub fn read_next_block<R: Read>(mut file: R) -> io::Result<Vec<u8>> {
    let mut sz_buf = [0u8;SIZE_SIZE];
    file.read_exact(&mut sz_buf)?;
    let sz = read_size(&sz_buf);
    // don't potentially consume all memory when reading a corrupt file
    assert!(sz < 20000000);
    let mut v : Vec<u8> = repeat(0).take(sz as usize).collect();
    file.read_exact(v.as_mut_slice())?;
    if (v.len() % 4) != 0 {
        let to_align = 4 - (v.len() % 4);
        let mut align = [0u8;4];
        file.read_exact(&mut align[0..to_align])?;
    }
    Ok(v)
}

// same as read_next_block, but when receiving EOF it will wrapped into returning None
pub fn read_next_block_or_eof<R: Read>(file: R) -> io::Result<Option<Vec<u8>>> {
    match read_next_block(file) {
        Err(err) => if err.kind() == io::ErrorKind::UnexpectedEof { Ok(None) } else { Err(err) },
        Ok(data) => Ok(Some(data)),
    }
}

impl<R: Read> Reader<R> {
    /// Return the next data block.
    ///
    /// note: any IO error raise runtime exception for now. will be changed soon.
    pub fn get_next(&mut self) -> Option<Vec<u8>> {
        // TODO: remove unwrap()
        let mdata = read_next_block_or_eof(&mut self.reader).unwrap();
        match mdata {
            None => {},
            Some(ref data) => {
                self.hash_context.input(data);
                self.pos += 4 + offset_align4(data.len() as u64);
            }
        };
        mdata
    }
}

impl<S: Read+Seek> Seeker<S> {
    /// Return the next data chunk if it exists
    /// on file EOF, None is returned
    pub fn get_next(&mut self) -> io::Result<Option<Vec<u8>>> {
        read_next_block_or_eof(&mut self.handle)
    }

    /// Return the data chunk at a specific offset, not that EOF is treated as a normal error here
    pub fn get_at_offset(&mut self, ofs: Offset) -> io::Result<Vec<u8>> {
        self.handle.seek(SeekFrom::Start(ofs))?;
        read_next_block(&mut self.handle)
    }
}

impl<R> Reader<R> {
    pub fn finalize(&mut self) -> PackHash {
        let mut packhash = [0u8;HASH_SIZE];
        self.hash_context.result(&mut packhash);
        packhash
    }
}

// A Writer for a specific pack that accumulate some numbers for reportings,
// index, blobs_hashes for index creation (in finalize)
pub struct Writer {
    tmpfile: TmpFile,
    index: indexfile::Index,
    pub nb_blobs: u32,
    pub pos: Offset, // offset in bytes of the current position (double as the current size of the pack)
    hash_context: blake2b::Blake2b, // hash of all the content of blocks without length or padding
}

impl Writer {
    pub fn init(mut tmpfile: TmpFile) -> Result<Self> {
        magic::write_header(&mut tmpfile, FILE_TYPE, VERSION)?;
        let idx = indexfile::Index::new();
        let ctxt = blake2b::Blake2b::new(32);
        Ok(Writer {
            tmpfile: tmpfile,
            index: idx,
            pos: magic::HEADER_SIZE as u64,
            nb_blobs: 0,
            hash_context: ctxt,
        })
    }

    pub fn append(&mut self, blockhash: &BlockHash, block: &[u8]) -> io::Result<()> {
        let len = block.len() as Size;
        let mut sz_buf = [0u8;SIZE_SIZE];
        write_size(&mut sz_buf, len);
        self.tmpfile.write_all(&sz_buf[..])?;
        self.tmpfile.write_all(block)?;
        self.hash_context.input(block);

        let pad = [0u8;SIZE_SIZE-1];
        let pad_bytes = if (len % 4 as u32) != 0 {
                            let pad_sz = 4 - len % 4;
                            self.tmpfile.write_all(&pad[0..pad_sz as usize])?;
                            pad_sz
                        } else { 0 };
        self.index.append(blockhash, self.pos);
        self.pos += 4 + len as u64 + pad_bytes as u64;
        self.nb_blobs += 1;
        Ok(())
    }

    pub fn finalize(mut self) -> io::Result<(TmpFile, PackHash, indexfile::Index)> {
        let mut packhash : PackHash = [0u8;HASH_SIZE];
        self.hash_context.result(&mut packhash);
        Ok((self.tmpfile, packhash, self.index))
    }
}
