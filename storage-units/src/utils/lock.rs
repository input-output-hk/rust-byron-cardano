use std::{
    error, fmt,
    fs::{self, OpenOptions},
    io::{self, Write},
    num,
    path::{Path, PathBuf},
    process, result,
};

/// the extension that will be added to the file, this will allow us to
/// lock a specific file.
const EXTENSION: &'static str = ".LOCK";

/// different lock errors that may happen when acquiring the lock
/// or when releasing the lock.
#[derive(Debug)]
pub enum Error {
    /// because the lock is using io operation, we might have error relating
    /// to IO.
    IoError(io::Error),
    /// we store the process ID in the file, if we cannot parse the process
    /// ID, this will be the error.
    ParseError(num::ParseIntError),
    /// tell the file was already locked and by whom (which process ID)
    AlreadyLocked(PathBuf, u32),
}
impl Error {
    /// convenient function to check if the error is because the file
    /// is already locked or if it is for other reasons.
    pub fn already_locked(&self) -> bool {
        match self {
            Error::AlreadyLocked(_, _) => true,
            _ => false,
        }
    }
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::IoError(e)
    }
}
impl From<num::ParseIntError> for Error {
    fn from(e: num::ParseIntError) -> Error {
        Error::ParseError(e)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(_) => write!(f, "I/O Error"),
            Error::ParseError(_) => write!(
                f,
                "Unable to read the lock file with the id of the locking process"
            ),
            Error::AlreadyLocked(path, id) => write!(f, "file {:?} already locked by {}", path, id),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::IoError(ref err) => Some(err),
            Error::ParseError(ref err) => Some(err),
            Error::AlreadyLocked(_, _) => None,
        }
    }
}

type Result<T> = result::Result<T, Error>;

/// Object which lifetime is bound to a file in the filesystem
///
/// i.e.: we are creating a `<filename>.LOCK` file along the given `filename`
/// in order to mark the file as locked. This is in order to prevent concurrent
/// access to a file that may be modified and which data may be corrupted if
/// concurrent writes happen.
///
/// The lock will be free when it drops out of scope.
///
#[derive(Debug)]
pub struct Lock {
    // the process ID associated to the current loc
    id: u32,
    // the path to the locked file
    path: PathBuf,
}

impl Lock {
    /// lock the given file
    ///
    /// this function will try to create a file `<path> '.LOCK'`.
    ///
    /// If the file already exists it will fail to create the lock.
    ///
    /// This is a non blocking function in the sense that: if a lock
    /// already exists, the function fail straight away and do not wait
    /// for the other process to release the `Lock`.
    ///
    pub fn lock<P: Into<PathBuf>>(path: P) -> Result<Self> {
        let lock = Lock {
            id: process::id(),
            path: path.into(),
        };
        lock.acquire()?;
        Ok(lock)
    }

    fn lock_path(&self) -> PathBuf {
        self.path.with_extension(EXTENSION)
    }

    fn acquire(&self) -> Result<()> {
        if let Some(dir) = self.lock_path().parent() {
            if !dir.is_dir() {
                fs::create_dir_all(dir)?;
            }
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .truncate(false)
            .open(self.lock_path())
            .or(Self::fail_with_lock(self.lock_path()))?;
        write!(file, "{}", self.id)?;
        Ok(())
    }

    fn fail_with_lock<A: Sized>(path: PathBuf) -> Result<A> {
        let id: u32 = fs::read_to_string(&path)?.parse()?;
        Err(Error::AlreadyLocked(path, id))
    }

    fn unlock(&self) -> Result<()> {
        Ok(fs::remove_file(self.lock_path())?)
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        self.unlock().unwrap();
    }
}

impl fmt::Display for Lock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.id == process::id() {
            write!(f, "file {:?} acquired", self.path)
        } else {
            write!(f, "file {:?} locked by {}", self.path, self.id)
        }
    }
}

impl AsRef<Path> for Lock {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}
