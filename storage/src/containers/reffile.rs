//! RefFile handling
//!
//! Contains a list of blockhash
//! hash containing all 0 are considered as the absence of a hash

use std::io::{Read,Seek,SeekFrom};
use std::io;
use std::fs;
use std::path::Path;
use types::{BlockHash, HASH_SIZE};

pub struct Reader {
    handle: fs::File,
}

impl Reader {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = fs::File::open(path)?;
        Ok(Reader { handle: file })
    }

    pub fn getref_at_index(&mut self, index: u32) -> io::Result<Option<BlockHash>> {
        let offset = index as usize * HASH_SIZE;
        self.handle.seek(SeekFrom::Start(offset as u64))?;
        self.next()
    }

    pub fn next(&mut self) -> io::Result<Option<BlockHash>> {
        let mut buf = [0;HASH_SIZE];
        self.handle.read_exact(&mut buf)?;
        // if all 0, then it's a empty slot otherwise return
        for v in buf.iter() {
            if *v != 0 {
                return Ok(Some(buf))
            }
        }
        return Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct Lookup(Vec<BlockHash>);

impl ::std::ops::Deref for Lookup {
    type Target = Vec<BlockHash>;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl ::std::ops::DerefMut for Lookup {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
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

    pub fn from_path<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut v = Lookup::new();
        let mut reader = Reader::open(path)?;
        loop {
            match reader.next() {
                Err(err) => {
                    if err.kind() == io::ErrorKind::UnexpectedEof { break } else { return Err(err) }
                },
                Ok(r) => {
                    match r {
                        None => v.append_missing_hash(),
                        Some(z) => v.append_hash(z),
                    }
                },
            }
        }
        Ok(v)
    }

    pub fn to_path<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut file = fs::File::create(path)?;
        self.write(&mut file)
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        for bh in self.iter() { writer.write_all(&bh[..])?; }
        Ok(())
    }

    pub fn append_missing_hash(&mut self) {
        let buf = [0;HASH_SIZE];
        self.0.push(buf);
    }

    pub fn append_hash(&mut self, hash: BlockHash) {
        self.0.push(hash);
    }
}