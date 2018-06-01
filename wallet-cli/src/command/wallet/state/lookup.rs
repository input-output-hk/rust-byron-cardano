use std::result;
use blockchain::{Block, BlockDate, HeaderHash, SlotId};
use wallet_crypto::hdwallet;
use wallet_crypto::bip44;
use wallet_crypto::tx::{TxId, TxOut};
use wallet_crypto::coin::Coin;

#[derive(Debug)]
pub enum Error {
    BlocksInvalidDate,
    BlocksInvalidHash,
    HdWalletError(hdwallet::Error),
    Bip44Error(bip44::Error),
}

impl From<hdwallet::Error> for Error {
    fn from(e: hdwallet::Error) -> Self { Error::HdWalletError(e) }
}
impl From<bip44::Error> for Error {
    fn from(e: bip44::Error) -> Self { Error::Bip44Error(e) }
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

#[derive(Clone,Debug)]
pub struct StatePtr {
    latest_addr: BlockDate,
    latest_known_hash: HeaderHash,
}

#[derive(Clone,Debug)]
pub struct State<T: AddrLookup> {
    ptr: StatePtr,
    lookup_struct: T,
    utxos: Utxos,
}

impl <T: AddrLookup> State<T> {
    /// update a given state with a set of blocks.
    /// 
    /// The blocks need to be in blockchain order,
    /// and correctly refer to each other, otherwise
    /// an error is emitted
    pub fn forward(&mut self, blocks: &[Block]) -> Result<()> {
        for block in blocks {
            let hdr = block.get_header();
            let date = hdr.get_blockdate();
            if self.ptr.latest_addr >= date {
                return Err(Error::BlocksInvalidDate)
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

                    let found_outputs = self.lookup_struct.lookup(&all_outputs[..]);
                    println!("found_outputs: {:?}", found_outputs)

                    // utxo
                },
            }

            // update the state
            self.ptr.latest_known_hash = hdr.compute_hash();
            self.ptr.latest_addr = date;
        }
        Ok(())
    }
}
