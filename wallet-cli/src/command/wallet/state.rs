use std::result;
use std::collections::{BTreeMap, VecDeque};
use blockchain::{Block, BlockDate, HeaderHash, SlotId};
use wallet_crypto::bip44;
use wallet_crypto::hdwallet;
use wallet_crypto::hdpayload;
use wallet_crypto::address::ExtendedAddr;
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

type Result<T> = result::Result<T, Error>;

#[derive(Clone,Debug)]
pub struct StatePtr {
    latest_addr: BlockDate,
    latest_known_hash: HeaderHash,
}

pub trait AddrLookup {
    /// given the lookup structure, return the list
    /// of matching addresses. note that for some
    /// algorithms, self mutates to optimise the next lookup query
    fn lookup(&mut self, addrs: &[&TxOut]) -> Result<Vec<TxOut>>;
}


#[derive(Clone,Debug)]
pub struct SequentialBip44Lookup {
    expected: BTreeMap<ExtendedAddr, bip44::Index>,
    gap_limit: u32,
    latest_to_generate: bip44::Index,
    parent_pk: hdwallet::XPub,
}

impl SequentialBip44Lookup {
    pub fn new(parent_pk: hdwallet::XPub, gap_limit: u32) -> Result<Self> {
        let mut expected = BTreeMap::new();
        let mut idx = bip44::Index::new(0)?;
        let first_gen_max = bip44::Index::new(gap_limit * 2)?;
        while idx < first_gen_max {
            let v = parent_pk.derive(idx.get_scheme_value())?;
            let a = ExtendedAddr::new_simple(v);
            expected.insert(a, idx);
            idx = idx.incr(1)?
        }

        let r = SequentialBip44Lookup {
            expected: expected,
            gap_limit: gap_limit,
            latest_to_generate: idx,
            parent_pk: parent_pk,
        };
        Ok(r)
    }

    // generate the next windows of addresses
    pub fn gen_next(&mut self) -> Result<()> {
        let next_latest = self.latest_to_generate.incr(self.gap_limit)?;

        while self.latest_to_generate < next_latest {
            let v = self.parent_pk.derive(self.latest_to_generate.get_scheme_value())?;
            let a = ExtendedAddr::new_simple(v);
            self.expected.insert(a, self.latest_to_generate);
            self.latest_to_generate = self.latest_to_generate.incr(1)?
        }
        Ok(())
    }
}

impl AddrLookup for SequentialBip44Lookup {
    fn lookup(&mut self, outs: &[&TxOut]) -> Result<Vec<TxOut>> {
        let mut found = Vec::new();
        let mut threshold = self.latest_to_generate.decr(self.gap_limit)?;
        for o in outs {
            match self.expected.remove(&o.address) {
                None => {},
                Some(idx) => {
                    if idx > threshold {
                        self.gen_next()?;
                        threshold = self.latest_to_generate.decr(self.gap_limit)?
                    }
                    found.push(o.clone().clone())
                },
            }
        }
        Ok(found)
    }
}

#[derive(Clone,Debug)]
pub struct RandomIndexLookup {
    key: hdpayload::HDKey,
}

impl RandomIndexLookup {
    pub fn new(root_pk: &hdwallet::XPub) -> Result<Self> {
        Ok(RandomIndexLookup { key: hdpayload::HDKey::new(root_pk) })
    }
}

impl AddrLookup for RandomIndexLookup {
    fn lookup(&mut self, outs: &[&TxOut]) -> Result<Vec<TxOut>> {
        let mut found = Vec::new();
        for o in outs {
            if is_our_ri_address(&self.key, &o.address.clone()) {
                found.push(o.clone().clone())
            }
        }
        Ok(found)
    }
}

// check if an address is an old style random index address
fn is_our_ri_address(key: &hdpayload::HDKey, addr: &ExtendedAddr) -> bool {
    match addr.attributes.derivation_path {
        None => false,
        Some(ref epath) => {
            match key.decrypt_path(epath) {
                None => false,
                Some(ref _path) => {
                    // TODO verify that the address really belongs to us
                    // by deriving the private key using the path
                    true
                },
            }
        },
    }
}

#[derive(Clone,Debug)]
pub struct Utxo {
    block_addr: SlotId,
    wallet_addr: bip44::Addressing,
    txid: TxId,
    coin: Coin,
}

type Utxos = Vec<Utxo>;

#[derive(Clone,Debug)]
pub struct StateAccount<T: AddrLookup> {
    lookup_struct: T,
}

#[derive(Clone,Debug)]
pub struct State<T: AddrLookup> {
    ptr: StatePtr,
    accounts: BTreeMap<u32, StateAccount<T>>,
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
                    for (_,a) in self.accounts.iter_mut() {
                        // TODO gather all inputs and compared with known UTXO for spending confirmation

                        // TODO compare utxo for spending
                        // gather all the outputs for reception
                        let mut all_outputs = Vec::new();
                        for txaux in txs.iter() {
                            for o in txaux.tx.outputs.iter() {
                                all_outputs.push(o)
                            }
                        }
                        let found_outputs = a.lookup_struct.lookup(&all_outputs[..]);
                        println!("found_outputs: {:?}", found_outputs)

                        // utxo
                    }
                },
            }

            // update the state
            self.ptr.latest_addr = date;
        }
        Ok(())
    }
}
