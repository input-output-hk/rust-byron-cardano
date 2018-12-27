use std::{
    error, fmt,
    fs::{self, OpenOptions},
    io::{self, Read},
    result,
};
use utils::lock::{self, Lock};
use utils::serialize::{io::write_length_prefixed, read_size, SIZE_SIZE};

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    EOF,
    NotFound,
    LockError(lock::Error),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            Error::EOF
        } else if e.kind() == io::ErrorKind::NotFound {
            Error::NotFound
        } else {
            Error::IoError(e)
        }
    }
}
impl From<lock::Error> for Error {
    fn from(e: lock::Error) -> Error {
        Error::LockError(e)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(_) => write!(f, "I/O Error"),
            Error::EOF => write!(f, "Unexpected End Of File"),
            Error::NotFound => write!(f, "Append file not found"),
            Error::LockError(_) => write!(f, "Lock Error"),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::IoError(ref err) => Some(err),
            Error::EOF => None,
            Error::NotFound => None,
            Error::LockError(ref err) => Some(err),
        }
    }
}

pub type Result<R> = result::Result<R, Error>;

/// Writer for an append only file
///
/// This structure is safe in the sense it tries to prevent
/// other instance of this structure to access the same file
/// via the `Lock` mechanism.
///
/// This object takes ownership of the given `Lock` file.
/// If you wish to take back ownership of the lock, use
/// the `close` function which releases the `Lock` yet close
/// the file descriptor opened for writing in the opened file.
///
/// Otherwise the Lock will be removed at the same time this
/// structure drop out of scope.
///
pub struct Writer {
    lock: Lock,
    file: fs::File,
}

impl Writer {
    /// open an already Locked file and take ownership of the lock
    ///
    /// When the Writer drop out of scope, the lock will be released too.
    /// Use `Writer::close` function to close the Writer yet keep the lock live.
    pub fn open(lock: Lock) -> Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(&lock)?;

        Ok(Writer { lock, file })
    }

    /// close the writer, yet take ownership of the lock, keeping the lock alive
    /// for other operations.
    ///
    pub fn close(self) -> Lock {
        self.lock
    }

    /// append a block of bytes
    ///
    /// the function will block until all the provided bytes are written
    /// The slice **must** contain all the bytes that needs to be written in the
    /// append only file.
    pub fn append_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        write_length_prefixed(&mut self.file, bytes)?;
        Ok(())
    }
}

/// Reader for an append only file
///
/// This structure is safe in the sense it tries to prevent
/// other instance of this structure to access the same file
/// via the `Lock` mechanism.
///
/// This object takes ownership of the given `Lock` file.
/// If you wish to take back ownership of the lock, use
/// the `close` function which releases the `Lock` yet close
/// the file descriptor opened for writing in the opened file.
///
/// Otherwise the Lock will be removed at the same time this
/// structure drop out of scope.
///
pub struct Reader {
    lock: Lock,
    file: fs::File,
}

impl Reader {
    /// open an already Locked file and take ownership of the lock
    ///
    /// When the Reader drop out of scope, the lock will be released too.
    /// Use `Reader::close` function to close the Reader yet keep the lock live.
    pub fn open(lock: Lock) -> Result<Self> {
        let file = OpenOptions::new().create(false).read(true).open(&lock)?;

        Ok(Reader { lock, file })
    }

    /// close the `Reader`, yet take ownership of the lock, keeping the lock alive
    /// for other operations.
    ///
    pub fn close(self) -> Lock {
        self.lock
    }

    /// get the next entry from the append only file
    /// returns `None` when we reach the end of the file.
    ///
    pub fn next(&mut self) -> Result<Option<Vec<u8>>> {
        match self.read_block_raw_next() {
            Err(Error::EOF) => Ok(None),
            Err(err) => Err(err),
            Ok(block_raw) => Ok(Some(block_raw)),
        }
    }

    #[inline]
    fn read_block_raw_next(&mut self) -> Result<Vec<u8>> {
        let mut sz_buf = [0u8; SIZE_SIZE];
        self.file.read_exact(&mut sz_buf)?;
        let sz = read_size(&sz_buf);
        let mut v = vec![0; sz as usize];
        self.file.read_exact(v.as_mut_slice())?;
        if (v.len() % 4) != 0 {
            let to_align = 4 - (v.len() % 4);
            let mut align = [0u8; 4];
            self.file.read_exact(&mut align[0..to_align])?;
        }
        Ok(v)
    }
}
