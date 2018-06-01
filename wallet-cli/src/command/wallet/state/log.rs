use storage::{Storage, append, lock::{self, Lock}};
use std::{fmt, result, path::{Path}};
use super::config;

pub enum Error {
    ConfigError(config::Error),
    LogFormatError(String),
    LockError(lock::Error),
    AppendError(append::Error)
}
impl From<lock::Error> for Error {
    fn from(e: lock::Error) -> Self { Error::LockError(e) }
}
impl From<append::Error> for Error {
    fn from(e: append::Error) -> Self { Error::AppendError(e) }
}
impl From<config::Error> for Error {
    fn from(e: config::Error) -> Self { Error::ConfigError(e) }
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Log {
    /* Todo */
    #[deprecated]
    Nothing
}
impl Log {
    fn serialise(&self) -> Vec<u8> {
        let buffer = Vec::new();
        match self {
            Log::Nothing => {},
        };
        buffer
    }

    fn deserisalise(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            Ok(Log::Nothing)
        } else {
            Err(Error::LogFormatError(format!("Nothing to parse yet")))
        }
    }
}
impl fmt::Display for Log {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Log::Nothing => write!(f, "<nothing yet>")
        }
    }
}

const WALLET_LOG_FILE : &'static str = "LOG";

pub struct LogLock(lock::Lock);
impl LogLock {
    /// function to acquire the lock on the log file of a given wallet
    ///
    /// The lock will hold as long as the lifetime of the returned object.
    pub fn acquire_wallet_log_lock<P: AsRef<Path>>(wallet_name: P) -> Result<Self> {
        let root = config::wallet_path(wallet_name)?;
        Ok(LogLock(Lock::lock(root.join(WALLET_LOG_FILE))?))
    }
}

/// Structure to read the Wallet Log one by one
pub struct LogReader(append::Reader);
impl LogReader {
    pub fn open(locked: LogLock) -> Result<Self> {
        Ok(LogReader(append::Reader::open(locked.0)?))
    }

    pub fn release_lock(self) -> LogLock { LogLock(self.0.close()) }

    pub fn next(&mut self) -> Result<Option<Log>> {
        match self.0.next()? {
            None => Ok(None),
            Some(bytes) => {
                let log = Log::deserisalise(&bytes)?;
                Ok(Some(log))
            }
        }
    }
}

pub struct LogWriter(append::Writer);
impl LogWriter {
    pub fn open(locked: LogLock) -> Result<Self> {
        Ok(LogWriter(append::Writer::open(locked.0)?))
    }

    pub fn release_lock(self) -> LogLock { LogLock(self.0.close()) }

    pub fn append(&mut self, log: &Log) -> Result<()> {
        info!("recording wallet log: {}", log);
        Ok(self.0.append_bytes(&log.serialise())?)
    }
}
