use wallet_crypto::hdwallet;
use wallet_crypto::hdpayload;
use wallet_crypto::address::ExtendedAddr;
use wallet_crypto::tx::{TxId, TxOut};
use super::lookup::{AddrLookup, Result};

#[derive(Clone,Debug)]
pub struct RandomIndexLookup {
    key: hdpayload::HDKey,
}

impl RandomIndexLookup {
    pub fn new(root_pk: &hdwallet::XPub) -> Result<Self> {
        Ok(RandomIndexLookup { key: hdpayload::HDKey::new(root_pk) })
    }

    fn one_of_mine(&self, addr: &ExtendedAddr) -> bool {
        match addr.attributes.derivation_path {
            None => false,
            Some(ref epath) => {
                match self.key.decrypt_path(epath) {
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
}

impl AddrLookup for RandomIndexLookup {
    fn lookup(&mut self, outs: &[&TxOut]) -> Result<Vec<TxOut>> {
        let mut found = Vec::new();
        for o in outs {
            if self.one_of_mine(&o.address) {
                found.push((*o).clone())
            }
        }
        Ok(found)
    }
}
