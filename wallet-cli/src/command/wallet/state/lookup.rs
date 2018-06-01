use std::{result, fmt, path::{Path, PathBuf}};
use blockchain::{Block, BlockDate, HeaderHash, SlotId};
use wallet_crypto::hdwallet;
use wallet_crypto::bip44;
use wallet_crypto::util::hex;
use wallet_crypto::tx::{TxId, TxOut};
use wallet_crypto::coin::Coin;

use super::log::{self, Log, LogReader, LogLock};

#[derive(Debug)]
pub enum Error {
    BlocksInvalidDate,
    BlocksInvalidHash,
    HdWalletError(hdwallet::Error),
    Bip44Error(bip44::Error),
    WalletLogError(log::Error),
}

impl From<hdwallet::Error> for Error {
    fn from(e: hdwallet::Error) -> Self { Error::HdWalletError(e) }
}
impl From<bip44::Error> for Error {
    fn from(e: bip44::Error) -> Self { Error::Bip44Error(e) }
}
impl From<log::Error> for Error {
    fn from(e: log::Error) -> Self { Error::WalletLogError(e) }
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Clone,Debug)]
pub struct Utxo {
    block_addr: SlotId,
    wallet_addr: bip44::Addressing,
    txid: TxId,
    coin: Coin,
}

pub type Utxos = Vec<Utxo>;

pub trait AddrLookup {
    /// given the lookup structure, return the list
    /// of matching addresses. note that for some
    /// algorithms, self mutates to optimise the next lookup query
    fn lookup(&mut self, addrs: &[&TxOut]) -> Result<Vec<TxOut>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePtr {
    latest_addr: Option<BlockDate>,
    latest_known_hash: HeaderHash,
}
impl fmt::Display for StatePtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref bd) = self.latest_addr {
            write!(f, "{}: {}", hex::encode(self.latest_known_hash.as_ref()), bd)
        } else {
            write!(f, "{}: Blockchain's genesis (The BigBang)", hex::encode(self.latest_known_hash.as_ref()))
        }
    }
}
impl StatePtr {
    pub fn new_before_genesis(before_genesis: HeaderHash) -> Self {
        StatePtr { latest_addr: None, latest_known_hash: before_genesis }
    }
    pub fn new(latest_addr: BlockDate, latest_known_hash: HeaderHash) -> Self {
        StatePtr { latest_addr: Some(latest_addr), latest_known_hash }
    }

    pub fn latest_block_date(&self) -> BlockDate {
        if let Some(ref date) = self.latest_addr {
            date.clone()
        } else {
            BlockDate::Genesis(0)
        }
    }
}

#[derive(Clone,Debug)]
pub struct State<T: AddrLookup> {
    pub ptr: StatePtr,
    pub lookup_struct: T,
    pub utxos: Utxos,
    pub wallet_name: PathBuf
}

impl <T: AddrLookup> State<T> {
    pub fn new(ptr: StatePtr, lookup_struct: T, utxos: Utxos, wallet_name: PathBuf) -> Self {
        State { ptr, lookup_struct, utxos, wallet_name }
    }

    pub fn load<P: AsRef<Path>>(wallet_name: P, mut ptr: StatePtr, mut lookup_struct: T) -> Result<Self> {
        let lock = LogLock::acquire_wallet_log_lock(wallet_name.as_ref())?;
        let utxos = Utxos::new();

        match LogReader::open(lock) {
            Err(log::Error::LogNotFound) => {},
            Err(err) => return Err(Error::from(err)),
            Ok(mut logs) => {
                while let Some(log) = logs.next()? {
                    match log {
                        Log::Checkpoint(known_ptr) => ptr = known_ptr,
                    }
                }
            }
        }

        Ok(Self::new(ptr, lookup_struct, utxos, wallet_name.as_ref().to_path_buf()))
    }

    /// update a given state with a set of blocks.
    ///
    /// The blocks need to be in blockchain order,
    /// and correctly refer to each other, otherwise
    /// an error is emitted
    pub fn forward(&mut self, blocks: &[Block]) -> Result<()> {
        let lock = LogLock::acquire_wallet_log_lock(&self.wallet_name)?;
        let mut log_writter = log::LogWriter::open(lock)?;
        for block in blocks {
            let hdr = block.get_header();
            let date = hdr.get_blockdate();
            if let Some(ref latest_addr) = self.ptr.latest_addr {
                if latest_addr >= &date {
                    return Err(Error::BlocksInvalidDate)
                }
            }
            // TODO verify the chain also

            match block.get_transactions() {
                None => {},
                Some(txs) => {
                    //for (_,a) in self.accounts.iter_mut() {
                    //}
                    // TODO gather all inputs and compared with known UTXO for spending confirmation
                    // TODO compare utxo for spending

                    // gather all the outputs for reception
                    let mut all_outputs = Vec::new();
                    for txaux in txs.iter() {
                        for o in txaux.tx.outputs.iter() {
                            all_outputs.push(o)
                        }
                    }

                    let found_outputs = self.lookup_struct.lookup(&all_outputs[..])?;
                    if ! found_outputs.is_empty() {
                        info!("found_outputs: {:?}", found_outputs)
                    }

                    // utxo
                },
            }
            // update the state
            self.ptr.latest_known_hash = hdr.compute_hash();
            self.ptr.latest_addr = Some(date.clone());

            if date.is_genesis() {
                log_writter.append(&Log::Checkpoint(self.ptr.clone()))?;
            }
        }
        Ok(())
    }
}
