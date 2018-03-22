use hdwallet::{Signature, XPub};
use address::ExtendedAddr;
use merkle;

struct Hash;
type TxId = Hash;

struct Coin(u64);
const MAX_COIN: Coin = Coin(45000000000000000);

type TODO = u8;
type ValidatorScript = TODO;
type RedeemerScript = TODO;
type RedeemPublicKey = TODO;
type RedeemSignature = TODO;

enum TxInWitness {
    /// signature of the `TxIn` with the associated `XPub`
    /// the `XPub` is the public key set in the AddrSpendingData
    PkWitness(XPub, Signature),
    ScriptWitness(ValidatorScript, RedeemerScript),
    RedeemWitness(RedeemPublicKey, RedeemSignature),
}

struct TxOut(ExtendedAddr, Coin);

struct TxIn(TxId, u32);

struct Tx {
    inputs: Vec<TxIn>,
    outputs: Vec<TxOut>, 
    // attributes: TxAttributes
    //
    // So far, there is no TxAttributes... the structure contains only the unparsed/unknown stuff
}

struct TxAux {
    tx: Tx,
    witnesses: Vec<TxInWitness>,
}

struct TxProof {
    number: u32,
    root: merkle::Root<Tx>,
    witnesses_hash: Hash,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TX: &'static [u8] = [/* TODO: insert TX here */];
    const BLOCK: &'static [u8] = [ /* TODO: insert Block here */ ];

    #[test]
    fn tx_decode() {
        // TODO test we can decode a cbor Tx
        unimplemented!()
    }
}
