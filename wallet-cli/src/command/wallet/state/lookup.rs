use std::{result, fmt, path::{Path, PathBuf}};
use std::collections::BTreeMap;
use blockchain::{Block, BlockDate, HeaderHash};
use wallet_crypto::hdwallet;
use wallet_crypto::hdpayload;
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
    // TODO remove from here
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

#[derive(Clone,Debug, Deserialize, Serialize)]
pub enum WalletAddr {
    Bip44(bip44::Addressing),
    Random(hdpayload::Path)
}

#[derive(Clone,Debug, Deserialize, Serialize)]
pub struct Utxo {
    pub block_addr: StatePtr,
    pub wallet_addr: WalletAddr,
    pub txid: TxId,
    pub offset: u32,
    pub coin: Coin,
}
impl fmt::Display for Utxo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?} received {}Ada-Lovelace in transaction id `{}.{}' ({})",
            self.wallet_addr,
            self.coin,
            self.txid,
            self.offset,
            self.block_addr
        )
    }
}

pub type Utxos = BTreeMap<TxId, Utxo>;

pub trait AddrLookup {
    /// given the lookup structure, return the list
    /// of matching addresses. note that for some
    /// algorithms, self mutates to optimise the next lookup query
    fn lookup(&mut self, ptr: &StatePtr, outs: &[(TxId, u32, &TxOut)]) -> Result<Vec<Utxo>>;

    /// when in the recovery phase of the implementor object, we will use this
    /// function to allow the tool to update its internal state with knowing
    /// an address is known of this wallet.
    ///
    fn acknowledge_address(&mut self, addr: &WalletAddr) -> Result<()>;
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
        let mut utxos = Utxos::new();

        match LogReader::open(lock) {
            Err(log::Error::LogNotFound) => {},
            Err(err) => return Err(Error::from(err)),
            Ok(mut logs) => {
                while let Some(log) = logs.next()? {
                    match log {
                        Log::Checkpoint(known_ptr) => ptr = known_ptr,
                        Log::ReceivedFund(utxo) => {
                            ptr = utxo.block_addr.clone();
                            utxos.insert(utxo.txid, utxo);
                        },
                        Log::SpentFund(utxo) => {
                            utxos.remove(&utxo.txid);
                        },
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
    pub fn forward(&mut self, blocks: &[Block]) -> Result<Vec<Log>> {
        let mut log_writer = Vec::new();
        for block in blocks {
            let hdr = block.get_header();
            let date = hdr.get_blockdate();
            if let Some(ref latest_addr) = self.ptr.latest_addr {
                if latest_addr >= &date {
                    return Err(Error::BlocksInvalidDate)
                }
            }
            let current_ptr = StatePtr {
                latest_known_hash: hdr.compute_hash(),
                latest_addr:       Some(hdr.get_blockdate())
            };
            // TODO verify the chain also

            match block.get_transactions() {
                None => {},
                Some(txs) => {
                    // cache if we have local utxos.
                    // note: that it might be expensive to call len() on btreemaps, could keep local value instead
                    let has_local_utxo = self.utxos.len() > 0;

                    // gather all the outputs for reception
                    let mut all_outputs = Vec::new();
                    let mut index = 0;
                    for txaux in txs.iter() {
                        let txid = txaux.tx.id();
                        // only do the input loop if we have local utxos
                        if has_local_utxo {
                            for txin in txaux.tx.inputs.iter() {
                                match self.utxos.remove(&txin.id) {
                                    None => {},
                                    Some(utxo) => {
                                        log_writer.push(Log::SpentFund(utxo))
                                    },
                                }
                            }
                        }
                        for o in txaux.tx.outputs.iter() {
                            all_outputs.push((txid, index, o))
                        }
                        index += 1;
                    }

                    let found_utxos = self.lookup_struct.lookup(&current_ptr, &all_outputs[..])?;
                    for utxo in found_utxos {
                        log_writer.push(Log::ReceivedFund(utxo.clone()));
                        self.utxos.insert(utxo.txid, utxo);
                    }

                    // utxo
                },
            }
            // update the state
            self.ptr = current_ptr;

            if date.is_genesis() {
                log_writer.push(Log::Checkpoint(self.ptr.clone()));
            }
        }
        Ok(log_writer)
    }

    pub fn forward_temp(&mut self, blocks: &[Block]) -> Result<()> {
        let lock = LogLock::acquire_wallet_log_lock(&self.wallet_name)?;
        let mut log_writer = log::LogWriter::open(lock)?;
        let events = self.forward(blocks)?;
        for ev in events {
            log_writer.append(&ev)?;
        }
        Ok(())
    }
}
