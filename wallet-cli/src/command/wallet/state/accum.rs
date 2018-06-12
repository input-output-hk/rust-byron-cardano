use wallet_crypto::tx::{TxIn, TxId, TxOut};
use super::lookup::{AddrLookup, Result, StatePtr, Utxo, WalletAddr};

#[derive(Clone,Debug)]
pub struct AccumLookup {}

impl AddrLookup for AccumLookup {
    fn lookup(&mut self, ptr: &StatePtr, outs: &[(TxId, u32, &TxOut)]) -> Result<Vec<Utxo>> {
        let mut found = Vec::new();
        for o in outs {
            let utxo = Utxo {
                block_addr: ptr.clone(),
                wallet_addr: WalletAddr::Accum,
                txin: TxIn { id: o.0.clone(), index: o.1},
                coin: o.2.value,
            };
            found.push(utxo)
        }
        Ok(found)
    }

    fn acknowledge_address(&mut self, _: &WalletAddr) -> Result<()> {
        Ok(())
    }
}
