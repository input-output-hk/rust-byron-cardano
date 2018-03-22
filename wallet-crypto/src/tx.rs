use std::marker::PhantomData;
use std::fmt;

use rcw::digest::Digest;
use rcw::blake2b::Blake2b;

use cbor;
use cbor::hs::{ToCBOR, FromCBOR};

use hdwallet::{Signature, XPub};
use address::ExtendedAddr;
use merkle;

/// Blake2b 256 bits
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Hash<T> {
    digest: [u8;32],
    _phantom: PhantomData<T>
}
impl<T> Hash<T> {
    pub fn new(buf: &[u8]) -> Self
    {
        let mut b2b = Blake2b::new(32);
        let mut out = [0;32];
        b2b.input(buf);
        b2b.result(&mut out);
        Self::from_bytes(out)
    }

    pub fn from_bytes(bytes :[u8;32]) -> Self { Hash { digest: bytes, _phantom: PhantomData } }
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != 32 { return None; }
        let mut buf = [0;32];

        buf[0..32].clone_from_slice(bytes);
        Some(Self::from_bytes(buf))
    }
}
impl<T> fmt::Display for Hash<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.digest.iter().for_each(|byte| {
            if byte < &0x10 {
                write!(f, "0{:x}", byte).unwrap()
            } else {
                write!(f, "{:x}", byte).unwrap()
            }
        });
        Ok(())
    }
}
impl<T> ToCBOR for Hash<T> {
    fn encode(&self, buf: &mut Vec<u8>) {
        cbor::encode::bs(&self.digest, buf)
    }
}
impl<T> FromCBOR for Hash<T> {
    fn decode(decoder: &mut cbor::decode::Decoder) -> cbor::decode::Result<Self> {
        let bs = decoder.bs()?;
        match Self::from_slice(&bs) {
            None => Err(cbor::decode::Error::Custom("invalid length for Hash")),
            Some(v) => Ok(v)
        }
    }
}

// TODO: this seems to be the hash of the serialisation CBOR of a given Tx.
// if this is confirmed, we need to make a proper type, wrapping it around
// to hash a `Tx` by serializing it cbor first.
pub type TxId = Hash<Tx>;

const MAX_COIN: u64 = 45000000000000000;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Coin(u64);
impl Coin {
    pub fn new(v: u64) -> Option<Self> {
        if v <= MAX_COIN { Some(Coin(v)) } else { None }
    }
}

type TODO = u8;
type ValidatorScript = TODO;
type RedeemerScript = TODO;
type RedeemPublicKey = TODO;
type RedeemSignature = TODO;

enum TxInWitness {
    /// signature of the `TxIn` with the associated `XPub`
    /// the `XPub` is the public key set in the AddrSpendingData
    PkWitness(XPub, Signature<Tx>),
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
    witnesses_hash: Hash<Vec<TxInWitness>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TX: &'static [u8] = &[/* TODO: insert TX here */];
    const BLOCK: &'static [u8] = &[ /* TODO: insert Block here */ ];

    #[test]
    fn tx_decode() {
        // TODO test we can decode a cbor Tx
        unimplemented!()
    }
}
