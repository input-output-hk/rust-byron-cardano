//! RefFile handling
//!
//! Contains a list of blockhash
//! hash containing all 0 are considered as the absence of a hash

use hash::{BlockHash, HASH_SIZE};
use std::fs;
use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use utils::error::{Result, StorageError};
use utils::magic;

const FILE_TYPE: magic::FileType = 0x52454653; // = REFS
const VERSION: magic::Version = 1;

pub struct Reader {
    handle: fs::File,
}

impl Reader {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = fs::File::open(path)?;
        magic::check_header(&mut file, FILE_TYPE, VERSION, VERSION)?;
        Ok(Reader { handle: file })
    }

    pub fn getref_at_index(&mut self, index: u32) -> io::Result<Option<BlockHash>> {
        let offset = (index as u64) * (HASH_SIZE as u64);
        self.handle.seek(SeekFrom::Start(offset))?;
        self.next()
    }

    /// Return the next hash, skipping empty slots, or None if we're
    /// at the end.
    pub fn next(&mut self) -> io::Result<Option<BlockHash>> {
        let mut buf = [0; HASH_SIZE];
        loop {
            // FIXME: buffer I/O.
            match self.handle.read_exact(&mut buf) {
                Err(ref err) if err.kind() == ::std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(err) => {
                    return Err(err);
                }
                Ok(()) => {
                    // if all 0, then it's a empty slot otherwise return
                    for v in buf.iter() {
                        if *v != 0 {
                            return Ok(Some(buf));
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lookup(Vec<BlockHash>);

impl ::std::ops::Deref for Lookup {
    type Target = Vec<BlockHash>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ::std::ops::DerefMut for Lookup {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<BlockHash>> for Lookup {
    fn from(other: Vec<BlockHash>) -> Self {
        Lookup(other)
    }
}

impl Lookup {
    pub fn new() -> Self {
        // TODO hardcoded size, maybe make it user parameter ?
        Lookup(Vec::with_capacity(21600))
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut v = Lookup::new();
        let mut reader = Reader::open(path)?;
        loop {
            match reader.next() {
                Err(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof {
                        break;
                    } else {
                        return Err(StorageError::IoError(err));
                    }
                }
                Ok(r) => match r {
                    None => v.append_missing_hash(),
                    Some(z) => v.append_hash(z),
                },
            }
        }
        Ok(v)
    }

    pub fn to_path<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut file = fs::File::create(path)?;
        self.write(&mut file)?;
        Ok(())
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> Result<()> {
        magic::write_header(writer, FILE_TYPE, VERSION)?;
        for bh in self.iter() {
            writer.write_all(&bh[..])?;
        }
        Ok(())
    }

    pub fn append_missing_hash(&mut self) {
        let buf = [0; HASH_SIZE];
        self.0.push(buf);
    }

    pub fn append_hash(&mut self, hash: BlockHash) {
        self.0.push(hash);
    }
}
