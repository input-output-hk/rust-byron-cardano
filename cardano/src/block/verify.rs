use block::*;
use self::normal::{VssCertificates, BlockSignature, ProxySignature, BodyProof, SscPayload};
use self::update;
use tx;
use coin;
use address;
use hash;
use config::{ProtocolMagic};
use cbor_event::{self, se};
use std::collections::{BTreeSet, HashSet};
use merkle;
use tags;
use hdwallet::{Signature};

#[derive(Debug)]
pub enum Error {
    BadBlockSig,
    BadTxWitness,
    BadUpdateProposalSig,
    BadUpdateVoteSig,
    BadVssCertSig,
    DuplicateInputs,
    DuplicateSigningKeys,
    DuplicateVSSKeys,
    EncodingError(cbor_event::Error),
    NoTxWitnesses,
    RedeemOutput,
    SelfSignedPSK,
    WrongBlockHash,
    WrongDelegationProof,
    WrongExtraDataProof,
    WrongGenesisProof,
    WrongMagic,
    WrongMpcProof,
    WrongTxProof,
    WrongUpdateProof,
    ZeroCoin,
}

impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self { Error::EncodingError(e) }
}

pub fn verify_block(protocol_magic: ProtocolMagic,
                    block_hash: &HeaderHash,
                    rblk: &RawBlock) -> Result<(), Error>
{
    let blk = rblk.decode()?;

    if block_hash != &blk.get_header().compute_hash() {
        return Err(Error::WrongBlockHash);
    }

    match blk {

        Block::GenesisBlock(blk) => {
            let hdr = &blk.header;

            if hdr.protocol_magic != protocol_magic {
                return Err(Error::WrongMagic);
            }

            // check body proof
            if hash::Blake2b256::new(&cbor!(&blk.body).unwrap()) != hdr.body_proof.0 {
                return Err(Error::WrongGenesisProof);
            }
        },

        Block::MainBlock(blk) => {

            let hdr = &blk.header;
            let body = &blk.body;

            if hdr.protocol_magic != protocol_magic {
                return Err(Error::WrongMagic);
            }

            // check tx proof

            if hdr.body_proof.tx.number as usize != body.tx.len() {
                return Err(Error::WrongTxProof);
            }

            // check tx merkle root
            let mut txs = vec![];
            for txaux in body.tx.iter() {
                txs.push(&txaux.tx);
            }
            let merkle_root = merkle::MerkleTree::new(&txs).get_root_hash();
            if merkle_root != hdr.body_proof.tx.root {
                return Err(Error::WrongTxProof);
            }

            // check tx witnesses hash
            let mut witnesses = vec![];
            for txaux in body.tx.iter() {
                let mut in_witnesses = vec![];
                for in_witness in txaux.witnesses.iter() {
                    in_witnesses.push(in_witness.clone());
                }
                witnesses.push(tx::TxWitness::new(in_witnesses));
            }
            if hash::Blake2b256::new(&cbor!(&tx::TxWitnesses::new(witnesses)).unwrap()) != hdr.body_proof.tx.witnesses_hash {
                return Err(Error::WrongTxProof);
            }

            // check mpc proof
            match hdr.body_proof.mpc {
                SscProof::Commitments(h1, h2) => {
                    match &body.ssc {
                        SscPayload::CommitmentsPayload(commitments, vss_certs) => {
                            if hash::Blake2b256::new(&cbor!(&commitments).unwrap()) != h1
                                || hash_vss_certs(&vss_certs) != h2
                            {
                                return Err(Error::WrongMpcProof);
                            }
                        },
                        _ => return Err(Error::WrongMpcProof)
                    };
                },
                SscProof::Openings(h1, h2) => {
                    match &body.ssc {
                        SscPayload::OpeningsPayload(openings_map, vss_certs) => {
                            if hash::Blake2b256::new(&cbor!(&openings_map).unwrap()) != h1
                                || hash_vss_certs(&vss_certs) != h2
                            {
                                return Err(Error::WrongMpcProof);
                            }
                        },
                        _ => return Err(Error::WrongMpcProof)
                    };
                },
                SscProof::Shares(h1, h2) => {
                    match &body.ssc {
                        SscPayload::SharesPayload(shares_map, vss_certs) => {
                            if hash::Blake2b256::new(&cbor!(&shares_map).unwrap()) != h1
                                || hash_vss_certs(&vss_certs) != h2
                            {
                                return Err(Error::WrongMpcProof);
                            }
                        },
                        _ => return Err(Error::WrongMpcProof)
                    };
                },
                SscProof::Certificate(h) => {
                    match &body.ssc {
                        SscPayload::CertificatesPayload(vss_certs) => {
                            if hash_vss_certs(&vss_certs) != h
                            {
                                return Err(Error::WrongMpcProof);
                            }
                        },
                        _ => return Err(Error::WrongMpcProof)
                    };
                },
            };

            // check delegation proof
            if hash::Blake2b256::new(&cbor!(&body.delegation).unwrap()) != hdr.body_proof.proxy_sk {
                return Err(Error::WrongDelegationProof);
            }

            // check update proof
            if hash::Blake2b256::new(&cbor!(&body.update).unwrap()) != hdr.body_proof.update {
                return Err(Error::WrongUpdateProof);
            }

            // check extra data proof
            if hash::Blake2b256::new(&cbor!(&blk.extra).unwrap()) != hdr.extra_data.extra_data_proof {
                return Err(Error::WrongExtraDataProof);
            }

            // check extra data

            // Note: the application name length restriction is
            // enforced by the SoftwareVersion constructor.

            // check consensus

            // FIXME: check slotid?

            match &hdr.consensus.block_signature {
                BlockSignature::Signature(_) => panic!("not implemented"),
                BlockSignature::ProxyLight(_) => panic!("not implemented"),
                BlockSignature::ProxyHeavy(proxy_sig) => {

                    // check against self-signed PSKs
                    if proxy_sig.psk.issuer_pk == proxy_sig.psk.delegate_pk {
                        return Err(Error::SelfSignedPSK);
                    }

                    // verify the signature
                    let to_sign = MainToSign {
                        previous_header: &hdr.previous_header,
                        body_proof: &hdr.body_proof,
                        slot: &hdr.consensus.slot_id,
                        chain_difficulty: &hdr.consensus.chain_difficulty,
                        extra_data: &hdr.extra_data,
                    };

                    if !verify_proxy_sig(protocol_magic, tags::SigningTag::MainBlockHeavy, proxy_sig, &to_sign) {
                        return Err(Error::BadBlockSig);
                    }
                }
            }

            // check tx
            body.tx.iter().try_for_each(|txaux| verify_txaux(protocol_magic, &txaux))?;

            // check ssc
            verify_vss_certificates(protocol_magic, body.ssc.get_vss_certificates())?;

            // check delegation

            // check update
            if let Some(proposal) = &body.update.proposal {
                proposal.verify(protocol_magic)?;
            }

            body.update.votes.iter().try_for_each(|vote| vote.verify(protocol_magic))?;
        }
    };

    Ok(())
}

fn hash_vss_certs(vss_certs: &VssCertificates) -> hash::Blake2b256 {
    let mut buf = vec![];
    vss_certs.serialize_for_proof(se::Serializer::new(&mut buf)).unwrap();
    hash::Blake2b256::new(&buf)
}

#[derive(Debug, Clone)]
struct MainToSign<'a>
{
    previous_header: &'a HeaderHash,
    body_proof: &'a BodyProof,
    slot: &'a EpochSlotId,
    chain_difficulty: &'a ChainDifficulty,
    extra_data: &'a HeaderExtraData,
}

impl<'a> cbor_event::se::Serialize for MainToSign<'a> {
    fn serialize<W: ::std::io::Write>(&self, serializer: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>> {
        serializer.write_array(cbor_event::Len::Len(5))?
            .serialize(&self.previous_header)?
            .serialize(&self.body_proof)?
            .serialize(&self.slot)?
            .serialize(&self.chain_difficulty)?
            .serialize(&self.extra_data)
    }
}

pub fn verify_proxy_sig<T>(
    protocol_magic: ProtocolMagic,
    tag: tags::SigningTag,
    proxy_sig: &ProxySignature,
    data: &T)
    -> bool
    where T: se::Serialize
{
    let mut buf = vec!['0' as u8, '1' as u8];

    buf.extend(proxy_sig.psk.issuer_pk.as_ref());

    se::Serializer::new(&mut buf)
        .serialize(&(tag as u8)).unwrap()
        .serialize(&protocol_magic).unwrap()
        .serialize(data).unwrap();

    proxy_sig.psk.delegate_pk.verify(
        &buf, &Signature::<()>::from_bytes(*proxy_sig.sig.to_bytes()))
}

pub fn verify_txaux(protocol_magic: ProtocolMagic, txaux: &tx::TxAux) -> Result<(), Error>
{
    // check that there are no duplicate inputs
    let mut inputs = BTreeSet::new();
    if !txaux.tx.inputs.iter().all(|x| inputs.insert(x)) {
        return Err(Error::DuplicateInputs);
    }

    // check that there are no duplicate outputs
    /*
    let mut outputs = BTreeSet::new();
    if !txaux.tx.outputs.iter().all(|x| outputs.insert(x.address.addr)) {
        return Err(Error::DuplicateOutputs);
    }
    */

    // check that all outputs have a non-zero amount
    if !txaux.tx.outputs.iter().all(|x| x.value > coin::Coin::zero()) {
        return Err(Error::ZeroCoin);
    }

    // Note: we don't need to check against MAX_COIN because Coin's
    // constructor already has.

    // check that none of the outputs are redeem addresses
    if txaux.tx.outputs.iter().any(|x| x.address.addr_type == address::AddrType::ATRedeem) {
        return Err(Error::RedeemOutput);
    }

    // TODO: check address attributes?

    // verify transaction witnesses
    if txaux.witnesses.is_empty() {
        return Err(Error::NoTxWitnesses);
    }

    txaux.witnesses.iter().try_for_each(|witness| {
        if !witness.verify_tx(protocol_magic, &txaux.tx) {
            return Err(Error::BadTxWitness);
        }
        Ok(())
    })?;

    Ok(())
}

pub fn verify_vss_certificates(protocol_magic: ProtocolMagic, vss_certs: &VssCertificates) -> Result<(), Error>
{
    // check that there are no duplicate VSS keys
    let mut vss_keys = BTreeSet::new();
    if !vss_certs.iter().all(|x| vss_keys.insert(x.vss_key.clone())) {
        return Err(Error::DuplicateVSSKeys);
    }

    // check that there are no duplicate signing keys
    let mut signing_keys = HashSet::new();
    if !vss_certs.iter().all(|x| signing_keys.insert(x.signing_key)) {
        return Err(Error::DuplicateSigningKeys);
    }

    // verify every certificate's signature
    for vss_cert in vss_certs.iter() {

        let mut buf = vec![];
        {
            let serializer = se::Serializer::new(&mut buf)
                .serialize(&(tags::SigningTag::VssCert as u8)).unwrap()
                .serialize(&protocol_magic).unwrap();
            let serializer = serializer.write_array(cbor_event::Len::Len(2))?;
            serializer
                .serialize(&vss_cert.vss_key).unwrap()
                .serialize(&vss_cert.expiry_epoch).unwrap();
        }

        if !vss_cert.signing_key.verify(&buf, &Signature::<()>::from_bytes(*vss_cert.signature.to_bytes())) {
            return Err(Error::BadVssCertSig);
        }
    }

    Ok(())
}

pub trait Verify {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error>;
}

impl Verify for update::UpdateProposal {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error>
    {
        // CoinPortion fields in block_version_mod and
        // block_version_mod.softfork_rule are checked by
        // CoinPortion::new().

        // SoftwareVersion is checked by SoftwareVersion::new().

        // SystemTags are checked by SystemTag::new().

        // Check the signature on the update proposal.
        let mut buf = vec![];

        let to_sign = update::UpdateProposalToSign {
            block_version: &self.block_version.clone(),
            block_version_mod: &self.block_version_mod.clone(),
            software_version: &self.software_version.clone(),
            data: &self.data.clone(),
            attributes: &self.attributes.clone(),
        };

        se::Serializer::new(&mut buf)
            .serialize(&(tags::SigningTag::USProposal as u8)).unwrap()
            .serialize(&protocol_magic).unwrap()
            .serialize(&to_sign).unwrap();

        if !self.from.verify(&buf, &Signature::<()>::from_bytes(*self.signature.to_bytes())) {
            return Err(Error::BadUpdateProposalSig);
        }

        Ok(())
    }
}

impl Verify for update::UpdateVote {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error>
    {
        let mut buf = vec![];
        se::Serializer::new(&mut buf)
            .serialize(&(tags::SigningTag::USVote as u8)).unwrap()
            .serialize(&protocol_magic).unwrap()
            .serialize(&(&self.proposal_id, &self.decision)).unwrap();

        if !self.key.verify(&buf, &Signature::<()>::from_bytes(*self.signature.to_bytes())) {
            return Err(Error::BadUpdateVoteSig);
        }

        Ok(())
    }
}
