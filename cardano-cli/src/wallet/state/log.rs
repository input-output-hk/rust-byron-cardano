use storage::{containers::append, utils::lock::{self, Lock}};
use std::{path::{PathBuf}, fmt, result};

use super::{ptr::{StatePtr}, utxo::{UTxO}};

use serde;
use serde_yaml;

#[derive(Debug)]
pub enum Error {
    LogNotFound,
    LogFormatError(String),
    LockError(lock::Error),
    AppendError(append::Error)
}
impl From<lock::Error> for Error {
    fn from(e: lock::Error) -> Self { Error::LockError(e) }
}
impl From<append::Error> for Error {
    fn from(e: append::Error) -> Self {
        match e {
            append::Error::NotFound => Error::LogNotFound,
            _ => Error::AppendError(e)
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Serialize, Deserialize)]
pub enum Log<A> {
    Checkpoint(StatePtr),
    ReceivedFund(UTxO<A>),
    SpentFund(UTxO<A>),
}
impl<A: serde::Serialize> Log<A> {
    fn serialise(&self) -> Vec<u8> {
        serde_yaml::to_vec(self).unwrap()
    }
}
impl<A> Log<A>
    where for<'de> A: serde::Deserialize<'de>
{
    fn deserisalise(bytes: &[u8]) -> Result<Self> {
        serde_yaml::from_slice(bytes).map_err(|e|
            Error::LogFormatError(format!("log format error: {:?}", e))
        )
    }
}
impl<A> Log<A>
{
    pub fn map<F, U>(self, f: F) -> Log<U>
        where F: FnOnce(A) -> U
    {
        match self {
            Log::Checkpoint(ptr)    => Log::Checkpoint(ptr),
            Log::ReceivedFund(utxo) => Log::ReceivedFund(utxo.map(f)),
            Log::SpentFund(utxo)    => Log::SpentFund(utxo.map(f)),
        }
    }
}
impl<A: fmt::Display> fmt::Display for Log<A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Log::Checkpoint(ptr) => write!(f, "Checkpoint at: {}", ptr),
            Log::ReceivedFund(utxo) => write!(f, "Received funds: {}", utxo),
            Log::SpentFund(utxo) => write!(f, "Spent funds: {}", utxo),
        }
    }
}

const WALLET_LOG_FILE : &'static str = "LOG";

pub struct LogLock(lock::Lock);
impl LogLock {
    /// function to acquire the lock on the log file of a given wallet
    ///
    /// The lock will hold as long as the lifetime of the returned object.
    pub fn acquire_wallet_log_lock(wallet_path: PathBuf) -> Result<Self> {
        Ok(LogLock(Lock::lock(wallet_path.join(WALLET_LOG_FILE))?))
    }

    pub fn delete_wallet_log_lock(self, wallet_path: PathBuf) -> ::std::io::Result<()> {
        let file = wallet_path.join(WALLET_LOG_FILE);
        ::std::fs::remove_file(file)
    }
}

/// Structure to read the Wallet Log one by one
pub struct LogReader(append::Reader);
impl LogReader {
    pub fn open(locked: LogLock) -> Result<Self> {
        Ok(LogReader(append::Reader::open(locked.0)?))
    }

    pub fn release_lock(self) -> LogLock { LogLock(self.0.close()) }

    pub fn into_iter<A>(self) -> LogIterator<A>
        where for<'de> A: serde::Deserialize<'de>
    {
        LogIterator {reader: self, _log_type: ::std::marker::PhantomData }
    }
    pub fn next<A>(&mut self) -> Result<Option<Log<A>>>
        where for<'de> A: serde::Deserialize<'de>
    {
        match self.0.next()? {
            None => Ok(None),
            Some(bytes) => {
                let log = Log::deserisalise(&bytes)?;
                Ok(Some(log))
            }
        }
    }
}

pub struct LogIterator<A> {
    reader: LogReader,
    _log_type: ::std::marker::PhantomData<A>
}
impl<A> Iterator for LogIterator<A>
    where for<'de> A: serde::Deserialize<'de>
{
    type Item = Result<Log<A>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.next() {
            Err(err) => Some(Err(err)),
            Ok(None) => None,
            Ok(Some(log)) => Some(Ok(log))
        }
    }
}

pub struct LogWriter(append::Writer);
impl LogWriter {
    pub fn open(locked: LogLock) -> Result<Self> {
        Ok(LogWriter(append::Writer::open(locked.0)?))
    }

    pub fn release_lock(self) -> LogLock { LogLock(self.0.close()) }

    pub fn append<A: serde::Serialize+fmt::Debug>(&mut self, log: &Log<A>) -> Result<()> {
        debug!("recording wallet log: {:?}", log);
        Ok(self.0.append_bytes(&log.serialise())?)
    }
}
