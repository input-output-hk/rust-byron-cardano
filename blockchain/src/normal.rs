use wallet_crypto::{tx, hdwallet, cbor, hash::{Blake2b256}};
use wallet_crypto::cbor::{ExtendedResult};
use wallet_crypto::config::{ProtocolMagic};
use std::{fmt};
use std::collections::linked_list::{Iter};
use std::collections::{LinkedList};

use types;
use types::{HeaderHash, HeaderExtraData, SlotId, ChainDifficulty};

#[derive(Debug, Clone)]
pub struct BodyProof {
    pub tx: tx::TxProof,
    pub mpc: types::SscProof,
    pub proxy_sk: Blake2b256, // delegation hash
    pub update: Blake2b256, // UpdateProof (hash of UpdatePayload)
}
impl BodyProof {
    pub fn new(tx: tx::TxProof, mpc: types::SscProof, proxy_sk: Blake2b256, update: Blake2b256) -> Self {
        BodyProof {
            tx: tx,
            mpc: mpc,
            proxy_sk: proxy_sk,
            update: update
        }
    }
}

impl cbor::CborValue for BodyProof {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::CborValue::encode(&self.tx),
            cbor::CborValue::encode(&self.mpc),
            cbor::CborValue::encode(&self.proxy_sk),
            cbor::CborValue::encode(&self.update),
        ])
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, tx)  = cbor::array_decode_elem(array, 0).embed("tx")?;
            let (array, mpc)  = cbor::array_decode_elem(array, 0).embed("mpc")?;
            let (array, proxy_sk)  = cbor::array_decode_elem(array, 0).embed("proxy_sk")?;
            let (array, update)  = cbor::array_decode_elem(array, 0).embed("update")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(BodyProof::new(tx, mpc, proxy_sk, update))
        }).embed("While decoding BodyProof")
    }
}

#[derive(Debug, Clone)]
pub struct TxPayload {
    txaux: LinkedList<tx::TxAux>
}
impl fmt::Display for TxPayload {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.txaux.is_empty() {
            return write!(f, "<no transactions>");
        }
        for txaux in self.txaux.iter() {
            writeln!(f, "{}", txaux)?;
        }
        write!(f, "")
    }
}
impl TxPayload {
    pub fn new(txaux: LinkedList<tx::TxAux>) -> Self {
        TxPayload { txaux: txaux }
    }
    pub fn empty() -> Self {
        TxPayload::new(LinkedList::new())
    }
    pub fn iter(&self) -> Iter<tx::TxAux> { self.txaux.iter() }
}
impl cbor::CborValue for TxPayload {
    fn encode(&self) -> cbor::Value {
        let mut l = LinkedList::new();
        for x in self.txaux.iter() {
            l.push_back(cbor::CborValue::encode(x));
        }
        cbor::CborValue::encode(&l)
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.iarray().and_then(|array| {
            let mut l = LinkedList::new();
            for i in array {
                l.push_back(cbor::CborValue::decode(i)?);
            }
            Ok(TxPayload::new(l))
        }).embed("While decoding TxPayload")
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    pub tx: TxPayload,
    pub ssc: cbor::Value,
    pub delegation: cbor::Value,
    pub update: cbor::Value
}
impl Body {
    pub fn new(tx: TxPayload, ssc: cbor::Value, dlg: cbor::Value, upd: cbor::Value) -> Self {
        Body { tx: tx, ssc: ssc, delegation: dlg, update: upd }
    }
}
impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.tx)
    }
}
impl cbor::CborValue for Body {
    fn encode(&self) -> cbor::Value {
        unimplemented!()
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, tx)  = cbor::array_decode_elem(array, 0).embed("tx")?;
            let (array, scc) = cbor::array_decode_elem(array, 0).embed("scc")?;
            let (array, dlg) = cbor::array_decode_elem(array, 0).embed("dlg")?;
            let (array, upd) = cbor::array_decode_elem(array, 0).embed("update")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(Body::new(tx, scc, dlg, upd))
        }).embed("While decoding main::Body")
    }
}

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub protocol_magic: ProtocolMagic,
    pub previous_header: HeaderHash,
    pub body_proof: BodyProof,
    pub consensus: Consensus,
    pub extra_data: HeaderExtraData
}
impl fmt::Display for BlockHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!( f
            , "Magic: 0x{:?} Previous Header: {}"
            , self.protocol_magic
            , self.previous_header
            )
    }
}
impl BlockHeader {
    pub fn new(pm: ProtocolMagic, pb: HeaderHash, bp: BodyProof, c: Consensus, ed: HeaderExtraData) -> Self {
        BlockHeader {
            protocol_magic: pm,
            previous_header: pb,
            body_proof: bp,
            consensus: c,
            extra_data: ed
        }
}
}
impl cbor::CborValue for BlockHeader {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::CborValue::encode(&self.protocol_magic),
            cbor::CborValue::encode(&self.previous_header),
            cbor::CborValue::encode(&self.body_proof),
            cbor::CborValue::encode(&self.consensus),
            cbor::CborValue::encode(&self.extra_data),
        ])
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, p_magic)    = cbor::array_decode_elem(array, 0).embed("protocol magic")?;
            let (array, prv_header) = cbor::array_decode_elem(array, 0).embed("Previous Header Hash")?;
            let (array, body_proof) = cbor::array_decode_elem(array, 0).embed("body proof")?;
            let (array, consensus)  = cbor::array_decode_elem(array, 0).embed("consensus")?;
            let (array, extra_data) = cbor::array_decode_elem(array, 0).embed("extra_data")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(BlockHeader::new(p_magic, prv_header, body_proof, consensus, extra_data))
        }).embed("While decoding a main::BlockHeader")
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub body: Body,
    pub extra: cbor::Value
}
impl Block {
    pub fn new(h: BlockHeader, b: Body, e: cbor::Value) -> Self {
        Block { header: h, body: b, extra: e }
    }
}
impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.header)?;
        write!(f, "{}", self.body)
    }
}
impl cbor::CborValue for Block {
    fn encode(&self) -> cbor::Value {
        let mut v = Vec::new();
        v.push(cbor::CborValue::encode(&self.header));
        v.push(cbor::CborValue::encode(&self.body));
        v.push(self.extra.clone());
        cbor::Value::Array(v)
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, header) = cbor::array_decode_elem(array, 0).embed("header")?;
            let (array, body)   = cbor::array_decode_elem(array, 0).embed("body")?;
            let (array, extra)  = cbor::array_decode_elem(array, 0).embed("extra")?;
            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(Block::new(header, body, extra))
        }).embed("While decoding block::Block")
    }
}

type SignData = ();

#[derive(Debug, Clone)]
pub enum BlockSignature {
    Signature(hdwallet::Signature<SignData>),
    ProxyLight(Vec<cbor::Value>),
    ProxyHeavy(Vec<cbor::Value>),
}
impl cbor::CborValue for BlockSignature {
    fn encode(&self) -> cbor::Value {
        match self {
            &BlockSignature::Signature(ref sig) =>
                cbor::Value::Array(vec![ cbor::Value::U64(0), cbor::CborValue::encode(sig) ]),
            &BlockSignature::ProxyLight(ref v) => {
                let mut r = Vec::new();
                r.push(cbor::Value::U64(1));
                r.extend_from_slice(v);
                cbor::Value::Array(r)
            },
            &BlockSignature::ProxyHeavy(ref v) => {
                let mut r = Vec::new();
                r.push(cbor::Value::U64(2));
                r.extend_from_slice(v);
                cbor::Value::Array(r)
            },
        }
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, code)  = cbor::array_decode_elem(array, 0).embed("enumeration code")?;
            match code {
                0u64 => {
                    let (array, sig) = cbor::array_decode_elem(array,0).embed("")?;
                    if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
                    Ok(BlockSignature::Signature(sig))
                },
                1u64 => { Ok(BlockSignature::ProxyLight(array)) },
                2u64 => { Ok(BlockSignature::ProxyHeavy(array)) },
                _    => { cbor::Result::array(array, cbor::Error::UnparsedValues) },
            }
        }).embed("While decoding main::BlockSignature")
    }
}

#[derive(Debug, Clone)]
pub struct Consensus {
    pub slot_id: SlotId,
    pub leader_key: hdwallet::XPub,
    pub chain_difficulty: ChainDifficulty,
    pub block_signature: BlockSignature,
}
impl cbor::CborValue for Consensus {
    fn encode(&self) -> cbor::Value {
        cbor::Value::Array(vec![
            cbor::CborValue::encode(&self.slot_id),
            cbor::CborValue::encode(&self.leader_key),
            cbor::CborValue::encode(&self.chain_difficulty),
            cbor::CborValue::encode(&self.block_signature),
        ])
    }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        value.array().and_then(|array| {
            let (array, slotid)  = cbor::array_decode_elem(array, 0).embed("slotid code")?;
            let (array, leaderkey)  = cbor::array_decode_elem(array, 0).embed("leader key")?;
            let (array, chain_difficulty) = cbor::array_decode_elem(array, 0).embed("chain difficulty")?;
            let (array, block_signature) = cbor::array_decode_elem(array, 0).embed("block signature")?;

            if ! array.is_empty() { return cbor::Result::array(array, cbor::Error::UnparsedValues); }
            Ok(Consensus {
                slot_id: slotid,
                leader_key: leaderkey,
                chain_difficulty: chain_difficulty,
                block_signature: block_signature,
            })
        }).embed("While decoding main::Consensus")
    }
}
