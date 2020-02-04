//! packfile format
//!
//! a pack file is a collection of blobs, prefixed by their 32 bits size in BE:
//!
//! SIZE (4 bytes BE)
//! DATA (SIZE bytes)
//! OPTIONAL ALIGNMENT? (of 0 to 3 bytes depending on SIZE)
//!
use cryptoxide::blake2b;
use cryptoxide::digest::Digest;
use hash::{BlockHash, PackHash, HASH_SIZE};
use indexfile;
use std::fs;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::iter::repeat;
use std::path::Path;
use utils::error::Result;
use utils::magic;
use utils::serialize::{io::write_length_prefixed, offset_align4, read_size, Offset, SIZE_SIZE};
use utils::tmpfile::TmpFile;

const FILE_TYPE: magic::FileType = 0x5041434b; // = PACK
const VERSION: magic::Version = 1;

/// A Stream Reader that also computes the hash of the sum of all data read
pub struct Reader<R> {
    reader: R,
    pos: Offset,
    hash_context: blake2b::Blake2b, // hash of all the content of blocks without length or padding
}

/// A pack reader that can seek in a packfile
pub struct Seeker<R> {
    handle: R,
}

impl Reader<fs::File> {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = fs::File::open(path)?;
        Reader::init(file)
    }
}

impl<R> Reader<R> {
    pub fn pos(&self) -> Offset {
        self.pos
    }
}

impl<R: Read> Reader<R> {
    pub fn init(mut r: R) -> Result<Self> {
        magic::check_header(&mut r, FILE_TYPE, VERSION, VERSION)?;
        let ctxt = blake2b::Blake2b::new(HASH_SIZE);
        Ok(Reader {
            reader: r,
            pos: 0,
            hash_context: ctxt,
        })
    }
}

impl Seeker<fs::File> {
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        magic::check_header(&mut file, FILE_TYPE, VERSION, VERSION)?;
        Ok(Seeker::from(file))
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
    let mut sz_buf = [0u8; SIZE_SIZE];
    file.read_exact(&mut sz_buf)?;
    let sz = read_size(&sz_buf);
    // don't potentially consume all memory when reading a corrupt file
    assert!(sz < 20000000, "read block of size: {}", sz);
    let mut v: Vec<u8> = repeat(0).take(sz as usize).collect();
    file.read_exact(v.as_mut_slice())?;
    if (v.len() % 4) != 0 {
        let to_align = 4 - (v.len() % 4);
        let mut align = [0u8; 4];
        file.read_exact(&mut align[0..to_align])?;
    }
    Ok(v)
}

// same as read_next_block, but when receiving EOF it will wrapped into returning None
pub fn read_next_block_or_eof<R: Read>(file: R) -> io::Result<Option<Vec<u8>>> {
    match read_next_block(file) {
        Err(err) => {
            if err.kind() == io::ErrorKind::UnexpectedEof {
                Ok(None)
            } else {
                Err(err)
            }
        }
        Ok(data) => Ok(Some(data)),
    }
}

impl<R: Read> Reader<R> {
    /// Reads the next data block if data are available in the source.
    /// If the source is at EOF, `None` is returned.
    ///
    /// # Errors
    /// I/O errors are returned in an `Err` value.
    pub fn next_block(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mdata = read_next_block_or_eof(&mut self.reader)?;
        match mdata {
            None => {}
            Some(ref data) => {
                self.hash_context.input(data);
                self.pos = self
                    .pos
                    .checked_add(4)
                    .unwrap()
                    .checked_add(offset_align4(data.len() as u64))
                    .unwrap();
            }
        };
        Ok(mdata)
    }
}

impl<S: Read + Seek> Seeker<S> {
    /// Return the next data chunk if it exists
    /// on file. On EOF, None is returned.
    pub fn next_block(&mut self) -> io::Result<Option<Vec<u8>>> {
        read_next_block_or_eof(&mut self.handle)
    }

    /// Return the data chunk at a specific offset.
    /// An EOF encountered before the specified offset is treated as a
    /// normal error.
    pub fn block_at_offset(&mut self, ofs: Offset) -> io::Result<Vec<u8>> {
        self.handle.seek(SeekFrom::Start(ofs))?;
        read_next_block(&mut self.handle)
    }
}

impl<R> Reader<R> {
    pub fn finalize(&mut self) -> PackHash {
        let mut packhash = [0u8; HASH_SIZE];
        self.hash_context.result(&mut packhash);
        packhash
    }
}

// A Writer for a specific pack that accumulate some numbers for reportings,
// index, blobs_hashes for index creation (in finalize)
pub struct Writer {
    tmpfile: TmpFile,
    index: indexfile::Index,
    nb_blobs: u32,
    pos: Offset, // offset in bytes of the current position (double as the current size of the pack)
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

    pub fn pos(&self) -> Offset {
        self.pos
    }

    pub fn append(&mut self, blockhash: &BlockHash, block: &[u8]) -> io::Result<()> {
        let bytes_written = write_length_prefixed(&mut self.tmpfile, block)?;
        self.hash_context.input(block);
        self.index.append(blockhash, self.pos);
        self.pos = self.pos.checked_add(bytes_written).unwrap();
        self.nb_blobs += 1;
        Ok(())
    }

    pub fn finalize(mut self) -> io::Result<(TmpFile, PackHash, indexfile::Index)> {
        let mut packhash: PackHash = [0u8; HASH_SIZE];
        self.hash_context.result(&mut packhash);
        Ok((self.tmpfile, packhash, self.index))
    }
}
