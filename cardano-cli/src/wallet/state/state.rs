use super::utxo::{UTxO, UTxOs};
use super::log::{Log};
use super::{lookup::{AddressLookup}, ptr::StatePtr};
use cardano::{tx::TxIn};
use std::{fmt};

#[derive(Debug)]
pub struct State<T: AddressLookup> {
    ptr: StatePtr,
    lookup_struct: T,
    utxos: UTxOs<T::AddressOutput>
}

impl<T: AddressLookup> State<T> {
    pub fn new(ptr: StatePtr, lookup_struct: T) -> Self {
        State { ptr: ptr, lookup_struct: lookup_struct, utxos: UTxOs::new() }
    }

    pub fn ptr<'a>(&'a self) -> &'a StatePtr { &self.ptr }

    /// update the wallet state with the given logs
    /// This function is for initializing the State by recovering the logs.
    ///
    pub fn update_with_logs<I: IntoIterator<Item = Log<T::AddressOutput>>>(&mut self, iter: I) -> Result<(), T::Error>
        where T::AddressOutput: fmt::Display
    {
        for log in iter {
            match log {
                Log::Checkpoint(known_ptr) => self.ptr = known_ptr,
                Log::ReceivedFund(utxo) => {
                    self.lookup_struct.acknowledge(&utxo.credited_address)?;
                    self.ptr = utxo.blockchain_ptr.clone();

                    if let Some(utxo) = self.utxos.insert(utxo.extract_txin(), utxo) {
                        error!("This UTxO was already in the UTxOs collection `{}'", utxo);
                        panic!("The Wallet LOG file seems corrupted");
                    };
                },
                Log::SpentFund(utxo) => {
                    match self.utxos.remove(&utxo.extract_txin()) {
                        Some(_) => { },
                        None    => {
                            error!("UTxO not in the known UTxOs collection `{}'", utxo);
                            panic!("The Wallet LOG file seems corrupted");
                        }
                    };
                    self.lookup_struct.acknowledge(&utxo.credited_address)?;
                    self.ptr = utxo.blockchain_ptr.clone();
                },
            }
        }
        Ok(())
    }

    pub fn forward_with_txins<I: IntoIterator<Item = TxIn>>(&mut self, iter: I) -> Result<Vec<Log<T::AddressOutput>>, T::Error>
        where T::AddressOutput: Clone
    {
        let mut events = Vec::new();
        for txin in iter {
            if let Some(utxo) = self.utxos.remove(&txin) {
                events.push(Log::SpentFund(utxo.clone()));
            }
        }
        Ok(events)
    }
    pub fn forward_with_utxos<I: IntoIterator<Item = UTxO<T::AddressInput>>>(&mut self, iter: I) -> Result<Vec<Log<T::AddressOutput>>, T::Error>
        where T::AddressOutput: Clone
    {
        let mut events = Vec::new();
        for utxo in iter {
            if let Some(utxo) = self.lookup_struct.lookup(utxo)? {
                self.ptr = utxo.blockchain_ptr.clone();
                events.push(Log::ReceivedFund(utxo.clone()));
                self.utxos.insert(utxo.extract_txin(), utxo);
            }
        }
        Ok(events)
    }
}
