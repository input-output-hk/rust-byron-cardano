use {address, tx, hdwallet, vss, hash::{Blake2b256}};
use config::{ProtocolMagic};
use std::{fmt};
use std::collections::{BTreeMap, btree_map};

use cbor_event::{self, de::RawCbor};
use super::types;
use super::types::{HeaderHash, HeaderExtraData, EpochSlotId, ChainDifficulty};

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

impl cbor_event::se::Serialize for BodyProof {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(4))?
            .serialize(&self.tx)?
            .serialize(&self.mpc)?
            .serialize(&self.proxy_sk)?
            .serialize(&self.update)
    }
}
impl cbor_event::de::Deserialize for BodyProof {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(4) {
            return Err(cbor_event::Error::CustomError(format!("Invalid BodyProof: recieved array of {:?} elements", len)));
        }
        let tx       = cbor_event::de::Deserialize::deserialize(raw)?;
        let mpc      = cbor_event::de::Deserialize::deserialize(raw)?;
        let proxy_sk = cbor_event::de::Deserialize::deserialize(raw)?;
        let update   = cbor_event::de::Deserialize::deserialize(raw)?;

        Ok(BodyProof::new(tx, mpc, proxy_sk, update))
    }
}

#[derive(Debug, Clone)]
pub struct TxPayload {
    txaux: Vec<tx::TxAux>
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
    pub fn new(txaux: Vec<tx::TxAux>) -> Self {
        TxPayload { txaux: txaux }
    }
    pub fn empty() -> Self {
        TxPayload::new(Vec::new())
    }
}
impl IntoIterator for TxPayload {
    type Item     = <Vec<tx::TxAux> as IntoIterator>::Item;
    type IntoIter = <Vec<tx::TxAux> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.txaux.into_iter()
    }
}
impl ::std::ops::Deref for TxPayload {
    type Target = [tx::TxAux];
    fn deref(&self) -> &Self::Target { self.txaux.deref() }
}
impl cbor_event::se::Serialize for TxPayload {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        cbor_event::se::serialize_indefinite_array(self.txaux.iter(), serializer)
    }
}
impl cbor_event::de::Deserialize for TxPayload {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let num_inputs = raw.array()?;
        assert_eq!(num_inputs, cbor_event::Len::Indefinite);
        let mut l = Vec::new();
        while {
            let t = raw.cbor_type()?;
            if t == cbor_event::Type::Special {
                let special = raw.special()?;
                assert_eq!(special, cbor_event::Special::Break);
                false
            } else {
                l.push(cbor_event::de::Deserialize::deserialize(raw)?);
                true
            }
        } {}

        Ok(TxPayload::new(l))
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    pub tx: TxPayload,
    pub ssc: SscPayload,
    pub delegation: cbor_event::Value, // TODO: decode
    pub update: cbor_event::Value // TODO: decode
}
impl Body {
    pub fn new(tx: TxPayload, ssc: SscPayload, dlg: cbor_event::Value, upd: cbor_event::Value) -> Self {
        Body { tx: tx, ssc: ssc, delegation: dlg, update: upd }
    }
}
impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.tx)
    }
}
impl cbor_event::se::Serialize for Body {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(4))?
            .serialize(&self.tx)?
            .serialize(&self.ssc)?
            .serialize(&self.delegation)?
            .serialize(&self.update)
    }
}
impl cbor_event::de::Deserialize for Body {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(4) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Body: recieved array of {:?} elements", len)));
        }
        let tx  = raw.deserialize()?;
        let scc = raw.deserialize()?;
        let dlg = raw.deserialize()?;
        let upd = raw.deserialize()?;

        Ok(Body::new(tx, scc, dlg, upd))
    }
}

#[derive(Debug, Clone)]
pub enum SscPayload {
    CommitmentsPayload(Commitments, VssCertificates),
    OpeningsPayload(OpeningsMap, VssCertificates),
    SharesPayload(SharesMap, VssCertificates),
    CertificatesPayload(VssCertificates),
}

impl SscPayload {
    pub fn get_vss_certificates(&self) -> &VssCertificates {
        match &self {
            SscPayload::CommitmentsPayload(_, vss) => vss,
            SscPayload::OpeningsPayload(_, vss) => vss,
            SscPayload::SharesPayload(_, vss) => vss,
            SscPayload::CertificatesPayload(vss) => vss
        }
    }
}

impl cbor_event::se::Serialize for SscPayload {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        match self {
            SscPayload::CommitmentsPayload(ref comms, ref cert) => {
                serializer.write_array(cbor_event::Len::Len(3))?
                    .write_unsigned_integer(0)?
                    .serialize(comms)?
                    .serialize(cert)
            },
            SscPayload::OpeningsPayload(ref openings, ref cert) => {
                serializer.write_array(cbor_event::Len::Len(3))?
                    .write_unsigned_integer(1)?
                    .serialize(openings)?
                    .serialize(cert)
            },
            SscPayload::SharesPayload(ref shares, ref cert) => {
                serializer.write_array(cbor_event::Len::Len(3))?
                    .write_unsigned_integer(2)?
                    .serialize(shares)?
                    .serialize(cert)
            },
            SscPayload::CertificatesPayload(ref cert) => {
                serializer.write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(3)?
                    .serialize(cert)
            },
        }
    }
}
impl cbor_event::de::Deserialize for SscPayload {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(2) && len != cbor_event::Len::Len(3) {
            return Err(cbor_event::Error::CustomError(format!("Invalid SscPayload: recieved array of {:?} elements", len)));
        }
        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            0 => {
                let comms  = raw.deserialize()?;
                let shares = raw.deserialize()?;
                Ok(SscPayload::CommitmentsPayload(comms, shares))
            },
            1 => {
                let openings = raw.deserialize()?;
                let vss      = raw.deserialize()?;
                Ok(SscPayload::OpeningsPayload(openings, vss))
            },
            2 => {
                let shares = raw.deserialize()?;
                let vss    = raw.deserialize()?;
                Ok(SscPayload::SharesPayload(shares, vss))
            },
            3 => {
                let vss    = raw.deserialize()?;
                Ok(SscPayload::CertificatesPayload(vss))
            },
            _ => {
                Err(cbor_event::Error::CustomError(format!("Unsupported BlockSignature: {}", sum_type_idx)))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Commitments(Vec<SignedCommitment>);
impl Commitments{
    pub fn iter(&self) -> ::std::slice::Iter<SignedCommitment> {
        self.0.iter()
    }
}
impl cbor_event::se::Serialize for Commitments {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        cbor_event::se::serialize_fixed_array(self.0.iter(), serializer.write_set_tag()?)
    }
}
impl cbor_event::de::Deserialize for Commitments {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.set_tag()?;
        Ok(Commitments(raw.deserialize()?))
    }
}

#[derive(Debug, Clone)]
pub struct SignedCommitment {
    pub public_key: hdwallet::XPub,
    pub commitment: Commitment,
    pub signature: vss::Signature,
}
impl cbor_event::se::Serialize for SignedCommitment {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(3))?
            .serialize(&self.public_key)?
            .serialize(&self.commitment)?
            .serialize(&self.signature)
    }
}
impl cbor_event::de::Deserialize for SignedCommitment {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(3) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Body: recieved array of {:?} elements", len)));
        }
        let public_key = raw.deserialize()?;
        let commitment = raw.deserialize()?;
        let signature  = raw.deserialize()?;

        Ok(SignedCommitment { public_key, commitment, signature})
    }
}

#[derive(Debug, Clone)]
pub struct Commitment {
    pub proof: SecretProof,
    pub shares: BTreeMap<vss::PublicKey, EncShare>,
}
impl cbor_event::se::Serialize for Commitment {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        let serializer = serializer.write_array(cbor_event::Len::Len(2))?;
        let serializer = cbor_event::se::serialize_fixed_map(self.shares.iter(), serializer)?;
        serializer.serialize(&self.proof)
    }
}
impl cbor_event::de::Deserialize for Commitment {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(2) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Body: recieved array of {:?} elements", len)));
        }
        let shares = raw.deserialize()?;
        let proof  = raw.deserialize()?;

        Ok(Commitment { shares, proof } )
    }
}

#[derive(Debug, Clone)]
pub struct SecretProof {
    pub extra_gen: cbor_event::Value, // TODO decode a http://hackage.haskell.org/package/pvss-0.2.0/docs/Crypto-SCRAPE.html#t:ExtraGen
    pub proof: cbor_event::Value, // TODO decode a http://hackage.haskell.org/package/pvss-0.2.0/docs/Crypto-SCRAPE.html#t:Proof
    pub parallel_proofs: cbor_event::Value, // TODO decode a http://hackage.haskell.org/package/pvss-0.2.0/docs/Crypto-SCRAPE.html#t:ParallelProofs
    pub commitments: Vec<cbor_event::Value>, // TODO decode a http://hackage.haskell.org/package/pvss-0.2.0/docs/Crypto-SCRAPE.html#t:Commitment
}
impl cbor_event::se::Serialize for SecretProof {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        let serializer = serializer.write_array(cbor_event::Len::Len(4))?
            .serialize(&self.extra_gen)?
            .serialize(&self.proof)?
            .serialize(&self.parallel_proofs)?;
        cbor_event::se::serialize_indefinite_array(self.commitments.iter(), serializer)
    }
}
impl cbor_event::de::Deserialize for SecretProof {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(4) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Body: recieved array of {:?} elements", len)));
        }
        let extra_gen       = raw.deserialize()?;
        let proof           = raw.deserialize()?;
        let parallel_proofs = raw.deserialize()?;
        let commitments     = raw.deserialize()?;

        Ok(SecretProof { extra_gen, proof, parallel_proofs, commitments} )
    }
}

// TODO: decode to
// http://hackage.haskell.org/package/pvss-0.2.0/docs/Crypto-SCRAPE.html#t:EncryptedSi
#[derive(Debug, Clone)]
pub struct EncShare(cbor_event::Value);
impl cbor_event::se::Serialize for EncShare {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.serialize(&self.0)
    }
}
impl cbor_event::de::Deserialize for EncShare {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(EncShare(raw.deserialize()?))
    }
}

// TODO: decode value in this map to
// http://hackage.haskell.org/package/pvss-0.2.0/docs/Crypto-SCRAPE.html#t:Secret
#[derive(Debug, Clone)]
pub struct OpeningsMap(BTreeMap<address::StakeholderId, cbor_event::Value>);
impl OpeningsMap{
    pub fn iter(&self) -> btree_map::Iter<address::StakeholderId, cbor_event::Value> {
        self.0.iter()
    }
}
impl cbor_event::se::Serialize for OpeningsMap {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        cbor_event::se::serialize_fixed_map(self.0.iter(), serializer)
    }
}
impl cbor_event::de::Deserialize for OpeningsMap {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(OpeningsMap(raw.deserialize()?))
    }
}

#[derive(Debug, Clone)]
pub struct SharesMap(
    BTreeMap<address::StakeholderId, SharesSubMap>,
);
pub type SharesSubMap = BTreeMap<address::StakeholderId, DecShare>;
impl SharesMap{
    pub fn iter(&self) -> btree_map::Iter<address::StakeholderId, SharesSubMap> {
        self.0.iter()
    }
}
impl cbor_event::se::Serialize for SharesMap {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        let mut serializer = serializer.write_map(cbor_event::Len::Len(self.0.len() as u64))?;
        for element in self.iter() {
            serializer = serializer.serialize(element.0)?;
            serializer = cbor_event::se::serialize_fixed_map(element.1.iter(), serializer)?;
        }
        Ok(serializer)
    }
}
impl cbor_event::de::Deserialize for SharesMap {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(SharesMap(raw.deserialize()?))
    }
}

// TODO: decode to
// https://hackage.haskell.org/package/pvss-0.2.0/docs/Crypto-SCRAPE.html#t:DecryptedShare
#[derive(Debug, Clone)]
pub struct DecShare(cbor_event::Value);
impl cbor_event::se::Serialize for DecShare {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.serialize(&self.0)
    }
}
impl cbor_event::de::Deserialize for DecShare {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Ok(DecShare(raw.deserialize()?))
    }
}

// TODO: after we properly decode VssCertificate.vss_key, change this struct to a
// BTreeMap<StakeholderId, VssCertificate> see
// https://github.com/input-output-hk/cardano-sl/blob/005076eb3434444a505c0fb150ea98e56e8bb3d9/core/src/Pos/Core/Ssc/VssCertificatesMap.hs#L36-L44
#[derive(Debug, Clone)]
pub struct VssCertificates(Vec<VssCertificate>);
impl VssCertificates {
    pub fn iter(&self) -> ::std::slice::Iter<VssCertificate> {
        self.0.iter()
    }

    // For historical reasons, SSC proofs are computed by hashing the
    // serialization of a map of StakeholderIds to VssCertificates
    // (where StakeholderId is computed from each VssCertificate's
    // signing key), rather than the serialization of a set of
    // VssCertificates that's actually stored in the block.
    pub fn serialize_for_proof<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        let mut hash = BTreeMap::<address::StakeholderId, &VssCertificate>::new();
        for vss_cert in self.0.iter() {
            hash.insert(address::StakeholderId::new(&vss_cert.signing_key), vss_cert);
        };
        cbor_event::se::serialize_fixed_map(hash.iter(), serializer)
    }
}
impl cbor_event::se::Serialize for VssCertificates {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        cbor_event::se::serialize_fixed_array(self.iter(), serializer.write_set_tag()?)
    }
}
impl cbor_event::de::Deserialize for VssCertificates {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        raw.set_tag()?;
        Ok(VssCertificates(raw.deserialize()?))
    }
}


#[derive(Debug, Clone)]
pub struct VssCertificate {
    pub vss_key: vss::PublicKey,
    pub expiry_epoch: types::EpochId,
    pub signature: vss::Signature,
    pub signing_key: hdwallet::XPub,
}
impl cbor_event::se::Serialize for VssCertificate {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(4))?
            .serialize(&self.vss_key)?
            .serialize(&self.expiry_epoch)?
            .serialize(&self.signature)?
            .serialize(&self.signing_key)
    }
}
impl cbor_event::de::Deserialize for VssCertificate {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(4) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Body: recieved array of {:?} elements", len)));
        }
        let vss_key      = raw.deserialize()?;
        let expiry_epoch = raw.unsigned_integer()? as u32;
        let signature    = raw.deserialize()?;
        let signing_key  = raw.deserialize()?;

        Ok(VssCertificate { vss_key, expiry_epoch, signature, signing_key })
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
impl cbor_event::se::Serialize for BlockHeader {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(5))?
            .serialize(&self.protocol_magic)?
            .serialize(&self.previous_header)?
            .serialize(&self.body_proof)?
            .serialize(&self.consensus)?
            .serialize(&self.extra_data)
    }
}
impl cbor_event::de::Deserialize for BlockHeader {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(5) {
            return Err(cbor_event::Error::CustomError(format!("Invalid BlockHeader: recieved array of {:?} elements", len)));
        }

        let p_magic    = cbor_event::de::Deserialize::deserialize(raw)?;
        let prv_header = cbor_event::de::Deserialize::deserialize(raw)?;
        let body_proof = cbor_event::de::Deserialize::deserialize(raw)?;
        let consensus  = cbor_event::de::Deserialize::deserialize(raw)?;
        let extra_data = cbor_event::de::Deserialize::deserialize(raw)?;

        Ok(BlockHeader::new(p_magic, prv_header, body_proof, consensus, extra_data))
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub body: Body,
    pub extra: cbor_event::Value // TODO: decode
}
impl Block {
    pub fn new(h: BlockHeader, b: Body, e: cbor_event::Value) -> Self {
        Block { header: h, body: b, extra: e }
    }
}
impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.header)?;
        write!(f, "{}", self.body)
    }
}
impl cbor_event::se::Serialize for Block {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(3))?
            .serialize(&self.header)?
            .serialize(&self.body)?
            .serialize(&self.extra)
    }
}
impl cbor_event::de::Deserialize for Block {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(3) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Block: recieved array of {:?} elements", len)));
        }
        let header = raw.deserialize()?;
        let body   = raw.deserialize()?;
        let extra  = raw.deserialize()?;
        Ok(Block::new(header, body, extra))
    }
}

type SignData = ();

type ProxyCert = hdwallet::Signature<()>;

#[derive(Debug, Clone)]
pub struct ProxySecretKey {
    pub omega: u64,
    pub issuer_pk: vss::PublicKey,
    pub delegate_pk: vss::PublicKey,
    pub cert: ProxyCert,
}

impl cbor_event::se::Serialize for ProxySecretKey {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(4))?
            .serialize(&self.omega)?
            .serialize(&self.issuer_pk)?
            .serialize(&self.delegate_pk)?
            .serialize(&self.cert)
    }
}

impl cbor_event::de::Deserialize for ProxySecretKey {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(4) {
            return Err(cbor_event::Error::CustomError(format!("Invalid ProxySecretKey: received array of {:?} elements", len)));
        }

        let omega = cbor_event::de::Deserialize::deserialize(raw)?;
        let issuer_pk = cbor_event::de::Deserialize::deserialize(raw)?;
        let delegate_pk = cbor_event::de::Deserialize::deserialize(raw)?;
        let cert = cbor_event::de::Deserialize::deserialize(raw)?;

        Ok(ProxySecretKey { omega, issuer_pk, delegate_pk, cert })
    }
}

#[derive(Debug, Clone)]
pub struct ProxySignature {
    pub psk: ProxySecretKey,
    pub sig: hdwallet::Signature<()>,
}

impl cbor_event::se::Serialize for ProxySignature {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(2))?
            .serialize(&self.psk)?
            .serialize(&self.sig)
    }
}

impl cbor_event::de::Deserialize for ProxySignature {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(2) {
            return Err(cbor_event::Error::CustomError(format!("Invalid ProxySignature: received array of {:?} elements", len)));
        }

        let psk = cbor_event::de::Deserialize::deserialize(raw)?;
        let sig = cbor_event::de::Deserialize::deserialize(raw)?;

        Ok(ProxySignature { psk, sig })
    }
}

#[derive(Debug, Clone)]
pub enum BlockSignature {
    Signature(hdwallet::Signature<SignData>),
    ProxyLight(Vec<cbor_event::Value>),
    ProxyHeavy(ProxySignature),
}
impl BlockSignature {
    pub fn to_bytes<'a>(&'a self) -> Option<&'a [u8;hdwallet::SIGNATURE_SIZE]> {
        match self {
            BlockSignature::Signature(s) => Some(s.to_bytes()),
            _ => None,
        }
    }
}
impl cbor_event::se::Serialize for BlockSignature {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        match self {
            &BlockSignature::Signature(ref sig) => {
                serializer.write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(0)?.serialize(sig)
            },
            &BlockSignature::ProxyLight(ref v) => {
                let serializer = serializer.write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(1)?;
                cbor_event::se::serialize_fixed_array(v.iter(), serializer)
            },
            &BlockSignature::ProxyHeavy(ref v) => {
                serializer.write_array(cbor_event::Len::Len(2))?
                    .write_unsigned_integer(2)?
                    .serialize(v)
            },
        }
    }
}
impl cbor_event::de::Deserialize for BlockSignature {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(2) {
            return Err(cbor_event::Error::CustomError(format!("Invalid BlockSignature: received array of {:?} elements", len)));
        }
        let sum_type_idx = raw.unsigned_integer()?;
        match sum_type_idx {
            0 => {
                Ok(BlockSignature::Signature(raw.deserialize()?))
            },
            1 => {
                Ok(BlockSignature::ProxyLight(raw.deserialize()?))
            },
            2 => {
                Ok(BlockSignature::ProxyHeavy(
                    cbor_event::de::Deserialize::deserialize(raw)?))
            },
            _ => {
                Err(cbor_event::Error::CustomError(format!("Unsupported BlockSignature: {}", sum_type_idx)))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Consensus {
    pub slot_id: EpochSlotId,
    pub leader_key: hdwallet::XPub,
    pub chain_difficulty: ChainDifficulty,
    pub block_signature: BlockSignature,
}
impl cbor_event::se::Serialize for Consensus {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(4))?
            .serialize(&self.slot_id)?
            .serialize(&self.leader_key)?
            .serialize(&self.chain_difficulty)?
            .serialize(&self.block_signature)
    }
}
impl cbor_event::de::Deserialize for Consensus {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(4) {
            return Err(cbor_event::Error::CustomError(format!("Invalid Consensus: recieved array of {:?} elements", len)));
        }
        let slot_id = cbor_event::de::Deserialize::deserialize(raw)?;
        let leader_key = cbor_event::de::Deserialize::deserialize(raw)?;
        let chain_difficulty = cbor_event::de::Deserialize::deserialize(raw)?;
        let block_signature = cbor_event::de::Deserialize::deserialize(raw)?;
        Ok(Consensus {slot_id, leader_key, chain_difficulty, block_signature })
    }
}
