use super::utxo::{UTxO, UTxOs};
use super::log::{Log};
use super::{lookup::{AddressLookup, Address}, ptr::StatePtr};
use cardano::{tx::TxIn, coin::{self, Coin}, address::ExtendedAddr};

#[derive(Debug)]
pub struct State<T: AddressLookup> {
    pub ptr: StatePtr,
    pub lookup_struct: T,
    pub utxos: UTxOs<Address>
}

impl<T: AddressLookup> State<T> {
    pub fn new(ptr: StatePtr, lookup_struct: T) -> Self {
        State { ptr: ptr, lookup_struct: lookup_struct, utxos: UTxOs::new() }
    }

    pub fn ptr<'a>(&'a self) -> &'a StatePtr { &self.ptr }

    pub fn total(&self) -> coin::Result<Coin> {
        self.utxos
            .iter()
            .map(|(_, v)| v.credited_value)
            .fold(Ok(Coin::zero()), |acc, v| {
                acc.and_then(|acc| acc + v)
            })
    }

    /// update the wallet state with the given logs
    /// This function is for initializing the State by recovering the logs.
    ///
    pub fn update_with_logs<I: IntoIterator<Item = Log<Address>>>(&mut self, iter: I) -> Result<(), T::Error>
    {
        for log in iter {
            match log {
                Log::Checkpoint(known_ptr) => self.ptr = known_ptr,
                Log::ReceivedFund(ptr, utxo) => {
                    self.lookup_struct.acknowledge(utxo.credited_address.clone())?;
                    self.ptr = ptr;

                    if let Some(utxo) = self.utxos.insert(utxo.extract_txin(), utxo) {
                        error!("This UTxO was already in the UTxOs collection `{}'", utxo);
                        panic!("The Wallet LOG file seems corrupted");
                    };
                },
                Log::SpentFund(ptr, utxo) => {
                    match self.utxos.remove(&utxo.extract_txin()) {
                        Some(_) => { },
                        None    => {
                            error!("UTxO not in the known UTxOs collection `{}'", utxo);
                            panic!("The Wallet LOG file seems corrupted");
                        }
                    };
                    self.lookup_struct.acknowledge(utxo.credited_address.clone())?;
                    self.ptr = ptr;
                },
            }
        }
        Ok(())
    }

    pub fn forward_with_txins<'a, I>(&mut self, iter: I) -> Result<Vec<Log<Address>>, T::Error>
        where I: IntoIterator<Item = (StatePtr, &'a TxIn)>
    {
        let mut events = Vec::new();
        for (ptr, txin) in iter {
            if let Some(utxo) = self.utxos.remove(&txin) {
                events.push(Log::SpentFund(ptr, utxo.clone()));
            }
        }
        Ok(events)
    }
    pub fn forward_with_utxos<I>(&mut self, iter: I) -> Result<Vec<Log<Address>>, T::Error>
        where I: IntoIterator<Item = (StatePtr, UTxO<ExtendedAddr>)>
    {
        let mut events = Vec::new();
        for (ptr, utxo) in iter {
            if let Some(utxo) = self.lookup_struct.lookup(utxo)? {
                self.ptr = ptr.clone();
                events.push(Log::ReceivedFund(ptr, utxo.clone()));
                self.utxos.insert(utxo.extract_txin(), utxo);
            }
        }
        Ok(events)
    }
}
