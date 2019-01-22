use self::normal::{BodyProof, VssCertificates};
use self::sign::{BlockSignature, MainToSign};
use self::update;
use address;
use block::*;
use cbor_event::{self, se};
use coin;
use config::ProtocolMagic;
use fee;
use hash;
use hdwallet::Signature;
use std::{
    collections::{BTreeSet, HashSet},
    error, fmt,
};
use tags;
use tx;

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
    UnexpectedWitnesses,
    MissingWitnesses,
    RedeemOutput,
    NoInputs,
    NoOutputs,
    SelfSignedPSK,
    WrongBlockHash,
    WrongDelegationProof,
    WrongExtraDataProof,
    WrongBoundaryProof,
    WrongMagic,
    WrongMpcProof,
    WrongRedeemTxId,
    WrongTxProof,
    WrongUpdateProof,
    ZeroCoin,

    // Used by verify_block_in_chain.
    WrongPreviousBlock(HeaderHash, HeaderHash), // actual, expected
    NonExistentSlot,
    BlockDateInPast,
    BlockDateInFuture,
    WrongSlotLeader,
    MissingUtxo,
    InputsTooBig,
    OutputsTooBig,
    OutputsExceedInputs,
    FeeError(fee::Error),
    AddressMismatch,
    DuplicateTxo,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            BadBlockSig => write!(f, "invalid block signature"),
            BadTxWitness => write!(f, "invalid transaction witness"),
            BadUpdateProposalSig => write!(f, "invalid update proposal signature"),
            BadUpdateVoteSig => write!(f, "invalid update vote signature"),
            BadVssCertSig => write!(f, "invalid VSS certificate signature"),
            DuplicateInputs => write!(f, "duplicated inputs"),
            DuplicateSigningKeys => write!(f, "duplicated signing keys"),
            DuplicateVSSKeys => write!(f, "duplicated VSS keys"),
            EncodingError(_error) => write!(f, "encoding error"),
            UnexpectedWitnesses => write!(f, "transaction has more witnesses than inputs"),
            MissingWitnesses => write!(f, "transaction has more inputs than witnesses"),
            RedeemOutput => write!(f, "invalid redeem output"),
            NoInputs => write!(f, "transaction has no inputs"),
            NoOutputs => write!(f, "transaction has no outputs"),
            SelfSignedPSK => write!(f, "invalid self signing PSK"),
            WrongBlockHash => write!(f, "block hash is invalid"),
            WrongDelegationProof => write!(f, "delegation proof is invalid"),
            WrongExtraDataProof => write!(f, "extra data proof is invalid"),
            WrongBoundaryProof => write!(f, "boundary proof is invalid"),
            WrongMagic => write!(f, "magic number is invalid"),
            WrongMpcProof => write!(f, "MPC proof is invalid"),
            WrongTxProof => write!(f, "transaction proof is invalid"),
            WrongUpdateProof => write!(f, "update proof is invalid"),
            ZeroCoin => write!(f, "output with no credited value"),
            WrongPreviousBlock(actual, expected) => write!(
                f,
                "block has parent {} while {} was expected",
                actual, expected
            ),
            NonExistentSlot => write!(f, "slot does not have a leader"),
            BlockDateInPast => write!(f, "block's slot or epoch is earlier than its parent"),
            BlockDateInFuture => write!(f, "block is in a future epoch"),
            WrongSlotLeader => write!(
                f,
                "block was not signed by the slot leader indicated in the genesis block"
            ),
            MissingUtxo => write!(
                f,
                "transaction spends an input that doesn't exist or has already been spent"
            ),
            InputsTooBig => write!(f, "sum of inputs exceeds limit"),
            OutputsTooBig => write!(f, "sum of outputs exceeds limit"),
            OutputsExceedInputs => write!(f, "sum of outputs is larger than sum of inputs and fee"),
            FeeError(_) => write!(f, "fee calculation failed"),
            WrongRedeemTxId => write!(f, "transaction input's ID does not match redeem public key"),
            AddressMismatch => write!(f, "transaction input witness does not match utxo address"),
            DuplicateTxo => write!(f, "transaction has an output that already exists"),
        }
    }
}

impl From<cbor_event::Error> for Error {
    fn from(e: cbor_event::Error) -> Self {
        Error::EncodingError(e)
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::EncodingError(ref error) => Some(error),
            Error::FeeError(ref error) => Some(error),
            _ => None,
        }
    }
}

pub trait Verify {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error>;
}

pub fn verify_block(block_hash: &HeaderHash, blk: &Block) -> Result<(), Error> {
    match blk {
        Block::BoundaryBlock(blk) => {
            blk.verify()?;
        }

        Block::MainBlock(blk) => {
            blk.verify()?;
        }
    };

    if block_hash != &blk.header().compute_hash() {
        return Err(Error::WrongBlockHash);
    }

    Ok(())
}

impl boundary::Block {
    fn verify(&self) -> Result<(), Error> {
        let hdr = &self.header;

        // check body proof
        if hash::Blake2b256::new(&cbor!(&self.body).unwrap()) != hdr.body_proof.0 {
            return Err(Error::WrongBoundaryProof);
        }

        Ok(())
    }
}

impl normal::Block {
    fn verify(&self) -> Result<(), Error> {
        let hdr = &self.header;
        let body = &self.body;

        // check extra data

        // Note: the application name length restriction is
        // enforced by the SoftwareVersion constructor.

        // check tx
        body.tx
            .iter()
            .try_for_each(|txaux| txaux.verify(hdr.protocol_magic))?;

        // check ssc
        body.ssc.get_vss_certificates().verify(hdr.protocol_magic)?;

        // check delegation
        // TODO

        // check update
        body.update.verify(hdr.protocol_magic)?;

        // compare the proofs generated from the body directly
        let proof = BodyProof::generate_from_body(&body);

        if proof.tx != hdr.body_proof.tx {
            return Err(Error::WrongTxProof);
        }
        if proof.mpc != hdr.body_proof.mpc {
            return Err(Error::WrongMpcProof);
        }
        if proof.delegation != hdr.body_proof.delegation {
            return Err(Error::WrongDelegationProof);
        }
        if proof.update != hdr.body_proof.update {
            return Err(Error::WrongUpdateProof);
        }

        // check extra data proof
        if hash::Blake2b256::new(&cbor!(&self.extra).unwrap()) != hdr.extra_data.extra_data_proof {
            return Err(Error::WrongExtraDataProof);
        }

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
                let to_sign = MainToSign::from_header(&hdr);

                if !to_sign.verify_proxy_sig(
                    hdr.protocol_magic,
                    tags::SigningTag::MainBlockHeavy,
                    proxy_sig,
                ) {
                    return Err(Error::BadBlockSig);
                }
            }
        }

        Ok(())
    }
}

impl Verify for update::UpdatePayload {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error> {
        if let Some(proposal) = &self.proposal {
            proposal.verify(protocol_magic)?;
        }

        self.votes
            .iter()
            .try_for_each(|vote| vote.verify(protocol_magic))?;

        Ok(())
    }
}

impl Verify for tx::TxAux {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error> {
        // check that there are inputs
        if self.tx.inputs.is_empty() {
            return Err(Error::NoInputs);
        }

        // check that there are outputs
        if self.tx.outputs.is_empty() {
            return Err(Error::NoOutputs);
        }

        // check that there are no duplicate inputs
        let mut inputs = BTreeSet::new();
        if !self.tx.inputs.iter().all(|x| inputs.insert(x)) {
            return Err(Error::DuplicateInputs);
        }

        // check that all outputs have a non-zero amount
        if !self.tx.outputs.iter().all(|x| x.value > coin::Coin::zero()) {
            return Err(Error::ZeroCoin);
        }

        // Note: we don't need to check against MAX_COIN because Coin's
        // constructor already has.

        // check that none of the outputs are redeem addresses
        if self
            .tx
            .outputs
            .iter()
            .any(|x| x.address.addr_type == address::AddrType::ATRedeem)
        {
            return Err(Error::RedeemOutput);
        }

        // TODO: check address attributes?

        // verify transaction witnesses
        if self.tx.inputs.len() < self.witness.len() {
            return Err(Error::UnexpectedWitnesses);
        }

        if self.tx.inputs.len() > self.witness.len() {
            return Err(Error::MissingWitnesses);
        }

        self.witness.iter().try_for_each(|in_witness| {
            if !in_witness.verify_tx(protocol_magic, &self.tx) {
                return Err(Error::BadTxWitness);
            }
            Ok(())
        })?;

        // verify that txids of redeem inputs correspond to the redeem pubkey
        for (txin, in_witness) in self.tx.inputs.iter().zip(self.witness.iter()) {
            if let tx::TxInWitness::RedeemWitness(pubkey, _) = in_witness {
                if tx::redeem_pubkey_to_txid(&pubkey, protocol_magic).0 != txin.id {
                    return Err(Error::WrongRedeemTxId);
                }
            }
        }

        Ok(())
    }
}

impl Verify for VssCertificates {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error> {
        // check that there are no duplicate VSS keys
        let mut vss_keys = BTreeSet::new();
        if !self.iter().all(|x| vss_keys.insert(x.vss_key.clone())) {
            return Err(Error::DuplicateVSSKeys);
        }

        // check that there are no duplicate signing keys
        let mut signing_keys = HashSet::new();
        if !self.iter().all(|x| signing_keys.insert(x.signing_key)) {
            return Err(Error::DuplicateSigningKeys);
        }

        // verify every certificate's signature
        for vss_cert in self.iter() {
            let mut buf = vec![];
            buf.push(tags::SigningTag::VssCert as u8);
            se::Serializer::new(&mut buf)
                .serialize(&protocol_magic)?
                .write_array(cbor_event::Len::Len(2))?
                .serialize(&vss_cert.vss_key)?
                .serialize(&vss_cert.expiry_epoch)?;

            if !vss_cert.signing_key.verify(
                &buf,
                &Signature::<()>::from_bytes(*vss_cert.signature.to_bytes()),
            ) {
                return Err(Error::BadVssCertSig);
            }
        }

        Ok(())
    }
}

impl Verify for update::UpdateProposal {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error> {
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

        buf.push(tags::SigningTag::USProposal as u8);

        se::Serializer::new(&mut buf)
            .serialize(&protocol_magic)?
            .serialize(&to_sign)?;

        if !self.from.verify(
            &buf,
            &Signature::<()>::from_bytes(*self.signature.to_bytes()),
        ) {
            return Err(Error::BadUpdateProposalSig);
        }

        Ok(())
    }
}

impl Verify for update::UpdateVote {
    fn verify(&self, protocol_magic: ProtocolMagic) -> Result<(), Error> {
        let mut buf = vec![];
        se::Serializer::new(&mut buf)
            .serialize(&(tags::SigningTag::USVote as u8))
            .unwrap()
            .serialize(&protocol_magic)
            .unwrap()
            .serialize(&(&self.proposal_id, &self.decision))
            .unwrap();

        if !self.key.verify(
            &buf,
            &Signature::<()>::from_bytes(*self.signature.to_bytes()),
        ) {
            return Err(Error::BadUpdateVoteSig);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use self::normal::DlgPayload;
    use address;
    use block::*;
    use cbor_event;
    use coin;
    use merkle;
    use std::fmt::Debug;
    use std::mem;
    use std::str::FromStr;

    #[test]
    #[should_panic]
    fn test_invalid_application_name() {
        SoftwareVersion::new(&"foosdksdlsdlksdlks", 123).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_too_long_system_tag() {
        SystemTag::new("abcdefghijk".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid_system_tag() {
        SystemTag::new("føø".to_string()).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid_coin_portion() {
        CoinPortion::new(1_000_000_000_000_001).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_invalid_coin() {
        coin::Coin::new(45_000_000_000_000_001).unwrap();
    }

    fn expect_error<T, Error>(res: &Result<T, Error>, expected: Error)
    where
        Error: Debug,
    {
        match res {
            Err(err) if mem::discriminant(&expected) == mem::discriminant(err) => {}
            Err(err) => panic!("Expected error {:?} but got {:?}", expected, err),
            Ok(_) => panic!("Expected error {:?} but succeeded", expected),
        }
    }

    #[test]
    fn test_verify() {
        let hash = HeaderHash::from_str(&HEADER_HASH1).unwrap();
        let rblk = RawBlock(BLOCK1.to_vec());
        let blk = rblk.decode().unwrap();
        assert!(verify_block(&hash, &blk).is_ok());

        let hash2 = HeaderHash::from_str(&HEADER_HASH2).unwrap();
        let rblk2 = RawBlock(BLOCK2.to_vec());
        let blk2 = rblk2.decode().unwrap();
        assert!(verify_block(&hash2, &blk2).is_ok());

        let hash3 = HeaderHash::from_str(&HEADER_HASH3).unwrap();
        let rblk3 = RawBlock(BLOCK3.to_vec());
        let blk3 = rblk3.decode().unwrap();
        assert!(verify_block(&hash3, &blk3).is_ok());

        // use a wrong header hash
        {
            expect_error(
                &verify_block(
                    &HeaderHash::from_str(
                        &"ae443ffffe52cc29de83312d2819b3955fc306ce65ae6aa5b26f1d3c76e91841",
                    )
                    .unwrap(),
                    &blk,
                ),
                Error::WrongBlockHash,
            );
        }

        // duplicate a tx input
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                let input = mblk.body.tx[0].tx.inputs[0].clone();
                mblk.body.tx[0].tx.inputs.push(input);
            }
            expect_error(&verify_block(&hash, &blk), Error::DuplicateInputs);
        }

        // invalidate a transaction witness
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.tx[0].tx.outputs[0].value = coin::Coin::new(123).unwrap();
            }
            expect_error(&verify_block(&hash, &blk), Error::BadTxWitness);
        }

        // create a zero output
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.tx[0].tx.outputs[0].value = coin::Coin::new(0).unwrap();
            }
            expect_error(&verify_block(&hash, &blk), Error::ZeroCoin);
        }

        // create a redeem output
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.tx[0].tx.outputs[0].address.addr_type = address::AddrType::ATRedeem;
            }
            expect_error(&verify_block(&hash, &blk), Error::RedeemOutput);
        }

        // remove the transaction input witness
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.tx[0].witness.clear();
            }
            expect_error(&verify_block(&hash, &blk), Error::MissingWitnesses);
        }

        // add a transaction input witness
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                let in_witness = mblk.body.tx[0].witness[0].clone();
                mblk.body.tx[0].witness.push(in_witness);
            }
            expect_error(&verify_block(&hash, &blk), Error::UnexpectedWitnesses);
        }

        // remove all transaction inputs
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.tx[0].tx.inputs.clear();
            }
            expect_error(&verify_block(&hash, &blk), Error::NoInputs);
        }

        // remove all transaction outputs
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.tx[0].tx.outputs.clear();
            }
            expect_error(&verify_block(&hash, &blk), Error::NoOutputs);
        }

        // invalidate the Merkle root by deleting a transaction
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.tx.pop();
            }
            expect_error(&verify_block(&hash, &blk), Error::WrongTxProof);
        }

        // invalidate the tx proof
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                // remove a tx
                mblk.body.tx.pop();
                // update the Merkle root
                let mut txs = vec![];
                for txaux in mblk.body.tx.iter() {
                    txs.push(&txaux.tx);
                }
                mblk.header.body_proof.tx.root = merkle::MerkleTree::new(&txs).get_root_hash();
            }
            expect_error(&verify_block(&hash, &blk), Error::WrongTxProof);
        }

        // invalidate the block signature
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.header.previous_header = HeaderHash::from_str(
                    &"aaaaaaaaaaaaaaa9de83312d2819b3955fc306ce65ae6aa5b26f1d3c76e91841",
                )
                .unwrap();
            }
            expect_error(&verify_block(&hash, &blk), Error::BadBlockSig);
        }

        // invalidate a VSS certificate
        {
            let mut blk = blk3.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                match &mut mblk.body.ssc {
                    normal::SscPayload::CommitmentsPayload(_, vss_certs) => {
                        vss_certs[0].expiry_epoch = 123;
                    }
                    _ => panic!(),
                }
            }
            expect_error(&verify_block(&hash, &blk), Error::BadVssCertSig);
        }

        // duplicate a VSS certificate
        {
            let mut blk = blk3.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                match &mut mblk.body.ssc {
                    normal::SscPayload::CommitmentsPayload(_, vss_certs) => {
                        let cert = vss_certs[0].clone();
                        vss_certs.push(cert);
                    }
                    _ => panic!(),
                }
            }
            expect_error(&verify_block(&hash, &blk), Error::DuplicateVSSKeys);
        }

        // invalidate the MPC proof
        {
            let mut blk = blk.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.ssc =
                    normal::SscPayload::CertificatesPayload(normal::VssCertificates::new(vec![]));
            }
            expect_error(&verify_block(&hash, &blk), Error::WrongMpcProof);
        }

        // invalidate the update proof
        {
            let mut blk = blk2.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.update.proposal = None;
            }
            expect_error(&verify_block(&hash2, &blk), Error::WrongUpdateProof);
        }

        // invalidate the update proposal signature
        {
            let mut blk = blk2.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body
                    .update
                    .proposal
                    .as_mut()
                    .unwrap()
                    .block_version
                    .major = 123;
            }
            expect_error(&verify_block(&hash2, &blk), Error::BadUpdateProposalSig);
        }

        // invalidate the update vote signature
        {
            let mut blk = blk2.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.update.votes[0].decision = false;
            }
            expect_error(&verify_block(&hash2, &blk), Error::BadUpdateVoteSig);
        }

        // invalidate the extra data proof
        {
            let mut blk = blk2.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.extra = cbor_event::Value::U64(123);
            }
            expect_error(&verify_block(&hash2, &blk), Error::WrongExtraDataProof);
        }

        // invalidate the delegation proof
        {
            let mut blk = blk2.clone();
            if let Block::MainBlock(mblk) = &mut blk {
                mblk.body.delegation = DlgPayload(cbor_event::Value::U64(123));
            }
            expect_error(&verify_block(&hash2, &blk), Error::WrongDelegationProof);
        }

        // add trailing data
        {
            let mut rblk = BLOCK1.to_vec();
            rblk.push(123);
            let rblk = RawBlock(rblk);
            expect_error(&rblk.decode(), cbor_event::Error::TrailingData);
        }

        // TODO: SelfSignedPSK, WrongBoundaryProof
    }

    // a block with 6 transactions
    const HEADER_HASH1: &str = "ae443ffffe52cc29de83312d2819b3955fc306ce65ae6aa5b26f1d3c76e91842";
    const BLOCK1: &'static [u8] = &[
        130, 1, 131, 133, 26, 37, 192, 15, 169, 88, 32, 143, 34, 167, 105, 182, 150, 66, 32, 255,
        10, 81, 134, 23, 91, 234, 166, 95, 163, 1, 164, 32, 9, 182, 196, 50, 7, 84, 107, 55, 169,
        7, 8, 132, 131, 6, 88, 32, 132, 17, 132, 183, 197, 80, 62, 62, 154, 179, 254, 210, 98, 186,
        81, 125, 195, 42, 41, 72, 189, 204, 52, 127, 25, 139, 229, 255, 49, 69, 186, 80, 88, 32,
        123, 58, 35, 252, 79, 123, 24, 14, 169, 86, 140, 116, 52, 47, 38, 0, 226, 218, 79, 71, 139,
        3, 51, 169, 109, 101, 84, 3, 227, 141, 64, 73, 131, 2, 88, 32, 211, 106, 38, 25, 166, 114,
        73, 70, 4, 225, 27, 180, 71, 203, 207, 82, 49, 233, 242, 186, 37, 194, 22, 145, 119, 237,
        201, 65, 189, 80, 173, 108, 88, 32, 211, 106, 38, 25, 166, 114, 73, 70, 4, 225, 27, 180,
        71, 203, 207, 82, 49, 233, 242, 186, 37, 194, 22, 145, 119, 237, 201, 65, 189, 80, 173,
        108, 88, 32, 175, 192, 218, 100, 24, 59, 242, 102, 79, 61, 78, 236, 114, 56, 213, 36, 186,
        96, 127, 174, 234, 178, 79, 193, 0, 235, 134, 29, 186, 105, 151, 27, 88, 32, 78, 102, 40,
        12, 217, 77, 89, 16, 114, 52, 155, 236, 10, 48, 144, 165, 58, 169, 69, 86, 46, 251, 109, 8,
        213, 110, 83, 101, 75, 14, 64, 152, 132, 130, 0, 25, 70, 230, 88, 64, 62, 106, 20, 205,
        246, 24, 147, 29, 211, 149, 178, 3, 73, 127, 25, 69, 51, 15, 136, 21, 216, 248, 219, 109,
        167, 253, 144, 243, 86, 203, 200, 200, 11, 59, 207, 111, 237, 37, 124, 127, 251, 217, 23,
        178, 107, 252, 206, 99, 225, 3, 153, 203, 55, 157, 21, 196, 141, 24, 58, 143, 216, 180, 13,
        208, 129, 25, 70, 11, 130, 2, 130, 132, 0, 88, 64, 62, 106, 20, 205, 246, 24, 147, 29, 211,
        149, 178, 3, 73, 127, 25, 69, 51, 15, 136, 21, 216, 248, 219, 109, 167, 253, 144, 243, 86,
        203, 200, 200, 11, 59, 207, 111, 237, 37, 124, 127, 251, 217, 23, 178, 107, 252, 206, 99,
        225, 3, 153, 203, 55, 157, 21, 196, 141, 24, 58, 143, 216, 180, 13, 208, 88, 64, 173, 14,
        253, 243, 52, 170, 85, 12, 231, 215, 139, 94, 52, 11, 69, 151, 165, 203, 30, 162, 162, 118,
        217, 44, 99, 68, 207, 44, 50, 146, 70, 134, 253, 36, 154, 111, 177, 94, 238, 211, 176, 97,
        164, 154, 236, 40, 153, 206, 131, 63, 161, 134, 226, 243, 1, 168, 69, 26, 11, 129, 234,
        218, 24, 138, 88, 64, 216, 232, 250, 40, 184, 199, 164, 83, 232, 204, 64, 229, 135, 246,
        253, 118, 165, 36, 240, 64, 60, 121, 168, 148, 4, 46, 255, 95, 172, 65, 59, 117, 215, 179,
        196, 254, 82, 71, 43, 10, 111, 81, 159, 109, 165, 237, 155, 215, 212, 64, 12, 207, 217,
        178, 36, 96, 91, 107, 24, 196, 45, 178, 34, 10, 88, 64, 148, 181, 185, 109, 37, 94, 145,
        104, 200, 211, 156, 35, 150, 249, 116, 16, 82, 105, 65, 135, 8, 21, 6, 250, 223, 107, 86,
        242, 170, 236, 67, 92, 90, 8, 28, 57, 140, 215, 231, 105, 76, 139, 162, 235, 20, 145, 7,
        187, 147, 105, 82, 238, 15, 147, 147, 51, 52, 158, 149, 161, 247, 193, 213, 0, 132, 131, 0,
        0, 0, 130, 106, 99, 97, 114, 100, 97, 110, 111, 45, 115, 108, 1, 160, 88, 32, 75, 169, 42,
        163, 32, 198, 10, 204, 154, 215, 185, 166, 79, 46, 218, 85, 196, 210, 236, 40, 230, 4, 250,
        241, 134, 112, 139, 79, 12, 78, 142, 223, 132, 159, 130, 131, 159, 130, 0, 216, 24, 88, 36,
        130, 88, 32, 108, 198, 215, 54, 227, 164, 57, 90, 202, 191, 174, 76, 124, 254, 64, 155,
        101, 216, 199, 198, 187, 249, 255, 133, 160, 189, 74, 149, 51, 75, 122, 95, 1, 255, 159,
        130, 130, 216, 24, 88, 66, 131, 88, 28, 144, 176, 21, 76, 224, 215, 155, 92, 142, 209, 145,
        153, 253, 40, 227, 170, 254, 92, 228, 248, 245, 69, 77, 224, 238, 254, 215, 0, 161, 1, 88,
        30, 88, 28, 36, 229, 182, 72, 91, 195, 10, 217, 140, 192, 7, 38, 244, 56, 236, 164, 211,
        188, 182, 218, 115, 7, 56, 183, 93, 150, 217, 40, 0, 26, 112, 108, 191, 145, 27, 0, 3, 141,
        126, 164, 183, 60, 185, 130, 130, 216, 24, 88, 66, 131, 88, 28, 192, 71, 176, 116, 80, 189,
        24, 226, 204, 144, 241, 91, 96, 130, 96, 40, 78, 60, 1, 91, 31, 187, 215, 234, 234, 61,
        107, 177, 161, 1, 88, 30, 88, 28, 215, 46, 197, 45, 192, 174, 78, 75, 244, 66, 44, 250,
        193, 173, 131, 206, 137, 35, 114, 224, 36, 73, 50, 241, 79, 234, 102, 101, 0, 26, 140, 146,
        138, 17, 26, 0, 12, 167, 9, 255, 160, 129, 130, 0, 216, 24, 88, 133, 130, 88, 64, 173, 110,
        28, 17, 240, 246, 6, 125, 3, 198, 142, 29, 40, 34, 174, 251, 116, 22, 202, 253, 137, 84,
        216, 168, 146, 58, 219, 159, 217, 252, 47, 236, 18, 82, 20, 154, 138, 52, 131, 82, 3, 184,
        210, 81, 25, 76, 199, 129, 123, 229, 218, 12, 238, 6, 2, 184, 141, 218, 168, 49, 137, 241,
        80, 45, 88, 64, 247, 55, 167, 42, 247, 248, 195, 214, 180, 90, 72, 174, 91, 13, 169, 235,
        180, 131, 122, 36, 151, 159, 175, 14, 168, 204, 144, 98, 254, 145, 204, 122, 239, 245, 109,
        84, 59, 0, 220, 105, 19, 52, 115, 248, 238, 88, 21, 94, 69, 183, 65, 54, 102, 222, 6, 166,
        28, 112, 97, 1, 46, 27, 190, 12, 130, 131, 159, 130, 0, 216, 24, 88, 36, 130, 88, 32, 226,
        183, 203, 87, 117, 93, 190, 173, 52, 242, 150, 78, 98, 43, 226, 115, 255, 137, 22, 77, 190,
        203, 22, 41, 33, 106, 1, 102, 165, 69, 87, 181, 0, 255, 159, 130, 130, 216, 24, 88, 66,
        131, 88, 28, 145, 159, 15, 214, 68, 21, 228, 57, 92, 128, 118, 91, 61, 204, 94, 170, 53,
        211, 172, 235, 35, 35, 255, 122, 25, 234, 158, 224, 161, 1, 88, 30, 88, 28, 36, 229, 182,
        72, 91, 195, 10, 199, 67, 241, 229, 38, 108, 200, 33, 18, 50, 145, 60, 78, 120, 41, 144,
        162, 17, 59, 237, 32, 0, 26, 131, 38, 93, 151, 27, 0, 3, 141, 126, 164, 167, 249, 114, 130,
        130, 216, 24, 88, 66, 131, 88, 28, 192, 71, 176, 116, 80, 189, 24, 226, 204, 144, 241, 91,
        96, 130, 96, 40, 78, 60, 1, 91, 31, 187, 215, 234, 234, 61, 107, 177, 161, 1, 88, 30, 88,
        28, 215, 46, 197, 45, 192, 174, 78, 75, 244, 66, 44, 250, 193, 173, 131, 206, 137, 35, 114,
        224, 36, 73, 50, 241, 79, 234, 102, 101, 0, 26, 140, 146, 138, 17, 26, 0, 12, 167, 9, 255,
        160, 129, 130, 0, 216, 24, 88, 133, 130, 88, 64, 254, 193, 225, 101, 26, 171, 169, 3, 145,
        51, 222, 37, 215, 17, 229, 126, 90, 101, 98, 192, 117, 170, 112, 60, 99, 40, 252, 233, 106,
        102, 220, 46, 2, 117, 239, 62, 22, 198, 51, 78, 165, 32, 208, 221, 170, 22, 143, 3, 221,
        180, 7, 163, 168, 27, 240, 53, 215, 41, 134, 17, 121, 190, 58, 202, 88, 64, 202, 87, 14,
        68, 103, 214, 49, 60, 54, 40, 233, 25, 157, 241, 76, 59, 7, 100, 9, 18, 141, 167, 162, 109,
        195, 121, 23, 209, 209, 0, 222, 6, 195, 5, 68, 153, 229, 160, 137, 107, 109, 7, 144, 173,
        228, 150, 213, 164, 192, 58, 149, 247, 235, 215, 155, 203, 186, 37, 153, 238, 87, 178, 77,
        11, 130, 131, 159, 130, 0, 216, 24, 88, 36, 130, 88, 32, 197, 52, 55, 126, 88, 214, 88,
        200, 71, 67, 172, 23, 157, 37, 23, 120, 84, 108, 2, 206, 155, 98, 112, 61, 187, 76, 123,
        151, 74, 221, 89, 250, 0, 255, 159, 130, 130, 216, 24, 88, 66, 131, 88, 28, 1, 78, 218,
        143, 48, 51, 55, 40, 157, 140, 182, 255, 85, 80, 24, 80, 122, 205, 159, 244, 119, 213, 227,
        6, 147, 178, 72, 9, 161, 1, 88, 30, 88, 28, 36, 229, 182, 72, 91, 195, 10, 137, 129, 23,
        31, 38, 161, 128, 185, 224, 19, 141, 208, 252, 223, 58, 194, 148, 96, 132, 14, 86, 0, 26,
        108, 156, 205, 67, 27, 0, 3, 141, 126, 164, 152, 182, 43, 130, 130, 216, 24, 88, 66, 131,
        88, 28, 192, 71, 176, 116, 80, 189, 24, 226, 204, 144, 241, 91, 96, 130, 96, 40, 78, 60, 1,
        91, 31, 187, 215, 234, 234, 61, 107, 177, 161, 1, 88, 30, 88, 28, 215, 46, 197, 45, 192,
        174, 78, 75, 244, 66, 44, 250, 193, 173, 131, 206, 137, 35, 114, 224, 36, 73, 50, 241, 79,
        234, 102, 101, 0, 26, 140, 146, 138, 17, 26, 0, 12, 167, 9, 255, 160, 129, 130, 0, 216, 24,
        88, 133, 130, 88, 64, 124, 79, 232, 149, 88, 45, 154, 114, 127, 232, 255, 228, 237, 69,
        231, 5, 93, 178, 33, 144, 185, 223, 227, 12, 112, 132, 165, 213, 139, 186, 254, 235, 24,
        174, 194, 151, 255, 40, 99, 75, 253, 174, 186, 52, 240, 186, 76, 110, 227, 108, 14, 142,
        131, 48, 141, 227, 63, 62, 170, 45, 206, 220, 44, 0, 88, 64, 158, 213, 155, 62, 239, 74,
        90, 219, 50, 238, 81, 158, 145, 100, 67, 117, 218, 46, 118, 59, 168, 210, 18, 66, 87, 132,
        151, 59, 153, 169, 104, 41, 35, 80, 39, 115, 61, 201, 197, 178, 188, 194, 164, 134, 216,
        90, 155, 72, 199, 176, 84, 22, 79, 54, 103, 226, 13, 210, 71, 37, 209, 238, 242, 5, 130,
        131, 159, 130, 0, 216, 24, 88, 36, 130, 88, 32, 177, 43, 137, 198, 100, 183, 185, 226, 216,
        119, 197, 8, 117, 238, 150, 7, 193, 187, 246, 99, 185, 147, 235, 180, 16, 4, 1, 167, 98,
        181, 196, 159, 0, 255, 159, 130, 130, 216, 24, 88, 66, 131, 88, 28, 165, 156, 6, 106, 168,
        112, 24, 94, 111, 104, 172, 23, 12, 206, 225, 135, 101, 192, 63, 100, 172, 155, 106, 105,
        39, 34, 70, 98, 161, 1, 88, 30, 88, 28, 36, 229, 182, 72, 91, 195, 10, 148, 59, 170, 170,
        38, 222, 190, 223, 115, 154, 238, 42, 11, 210, 184, 10, 211, 94, 81, 232, 157, 0, 26, 111,
        21, 178, 226, 27, 0, 3, 141, 126, 164, 137, 114, 228, 130, 130, 216, 24, 88, 66, 131, 88,
        28, 192, 71, 176, 116, 80, 189, 24, 226, 204, 144, 241, 91, 96, 130, 96, 40, 78, 60, 1, 91,
        31, 187, 215, 234, 234, 61, 107, 177, 161, 1, 88, 30, 88, 28, 215, 46, 197, 45, 192, 174,
        78, 75, 244, 66, 44, 250, 193, 173, 131, 206, 137, 35, 114, 224, 36, 73, 50, 241, 79, 234,
        102, 101, 0, 26, 140, 146, 138, 17, 26, 0, 12, 167, 9, 255, 160, 129, 130, 0, 216, 24, 88,
        133, 130, 88, 64, 132, 210, 141, 115, 144, 209, 117, 171, 43, 238, 127, 137, 1, 51, 150,
        50, 228, 207, 29, 238, 116, 14, 158, 234, 158, 201, 15, 77, 169, 114, 195, 118, 216, 156,
        165, 51, 189, 250, 220, 164, 43, 10, 80, 54, 162, 147, 126, 49, 143, 26, 225, 89, 195, 73,
        62, 234, 236, 175, 224, 192, 47, 86, 87, 80, 88, 64, 56, 152, 211, 64, 79, 0, 27, 90, 179,
        232, 233, 90, 142, 40, 190, 131, 138, 186, 45, 214, 185, 109, 99, 10, 48, 143, 172, 32,
        162, 102, 236, 114, 8, 85, 113, 167, 86, 125, 37, 10, 216, 209, 225, 56, 145, 250, 238, 72,
        65, 159, 64, 116, 152, 54, 246, 244, 105, 22, 122, 100, 226, 252, 43, 14, 130, 131, 159,
        130, 0, 216, 24, 88, 36, 130, 88, 32, 197, 191, 133, 248, 159, 204, 238, 45, 12, 19, 121,
        115, 0, 246, 29, 91, 170, 175, 223, 224, 207, 207, 97, 197, 39, 79, 150, 33, 78, 113, 51,
        18, 0, 255, 159, 130, 130, 216, 24, 88, 66, 131, 88, 28, 67, 34, 50, 19, 75, 254, 224, 173,
        51, 244, 236, 3, 174, 193, 93, 214, 9, 42, 219, 59, 241, 72, 215, 159, 200, 180, 87, 23,
        161, 1, 88, 30, 88, 28, 36, 229, 182, 72, 91, 195, 10, 187, 105, 119, 199, 38, 212, 248,
        50, 220, 52, 103, 178, 15, 244, 49, 155, 55, 223, 72, 87, 217, 0, 26, 5, 108, 105, 49, 27,
        0, 3, 141, 126, 164, 122, 47, 157, 130, 130, 216, 24, 88, 66, 131, 88, 28, 192, 71, 176,
        116, 80, 189, 24, 226, 204, 144, 241, 91, 96, 130, 96, 40, 78, 60, 1, 91, 31, 187, 215,
        234, 234, 61, 107, 177, 161, 1, 88, 30, 88, 28, 215, 46, 197, 45, 192, 174, 78, 75, 244,
        66, 44, 250, 193, 173, 131, 206, 137, 35, 114, 224, 36, 73, 50, 241, 79, 234, 102, 101, 0,
        26, 140, 146, 138, 17, 26, 0, 12, 167, 9, 255, 160, 129, 130, 0, 216, 24, 88, 133, 130, 88,
        64, 71, 40, 206, 101, 117, 227, 73, 29, 62, 220, 227, 48, 200, 234, 35, 12, 100, 111, 92,
        8, 126, 13, 187, 196, 142, 74, 70, 25, 115, 4, 88, 60, 153, 183, 119, 70, 117, 246, 47, 26,
        255, 206, 63, 227, 228, 201, 214, 236, 57, 26, 161, 187, 102, 245, 95, 40, 182, 25, 198,
        67, 255, 92, 222, 161, 88, 64, 121, 235, 196, 61, 155, 4, 238, 142, 127, 63, 22, 115, 190,
        164, 92, 110, 194, 21, 70, 16, 148, 87, 172, 49, 173, 69, 224, 230, 126, 148, 44, 162, 100,
        95, 166, 11, 207, 2, 174, 39, 156, 162, 243, 248, 197, 62, 170, 249, 197, 200, 206, 150,
        224, 1, 84, 131, 102, 135, 83, 187, 232, 180, 139, 11, 130, 131, 159, 130, 0, 216, 24, 88,
        36, 130, 88, 32, 128, 248, 175, 116, 198, 26, 218, 158, 166, 17, 59, 106, 239, 189, 30,
        226, 70, 193, 243, 16, 109, 134, 11, 55, 136, 72, 2, 114, 235, 93, 139, 195, 0, 255, 159,
        130, 130, 216, 24, 88, 66, 131, 88, 28, 15, 103, 36, 190, 63, 53, 108, 157, 121, 212, 16,
        52, 200, 169, 134, 136, 218, 232, 137, 240, 92, 18, 60, 186, 254, 250, 162, 42, 161, 1, 88,
        30, 88, 28, 36, 229, 182, 72, 91, 195, 10, 199, 218, 249, 165, 38, 8, 40, 227, 120, 226,
        206, 193, 146, 251, 246, 24, 123, 54, 18, 60, 17, 0, 26, 143, 121, 53, 212, 27, 0, 3, 141,
        126, 164, 106, 236, 86, 130, 130, 216, 24, 88, 66, 131, 88, 28, 192, 71, 176, 116, 80, 189,
        24, 226, 204, 144, 241, 91, 96, 130, 96, 40, 78, 60, 1, 91, 31, 187, 215, 234, 234, 61,
        107, 177, 161, 1, 88, 30, 88, 28, 215, 46, 197, 45, 192, 174, 78, 75, 244, 66, 44, 250,
        193, 173, 131, 206, 137, 35, 114, 224, 36, 73, 50, 241, 79, 234, 102, 101, 0, 26, 140, 146,
        138, 17, 26, 0, 12, 167, 9, 255, 160, 129, 130, 0, 216, 24, 88, 133, 130, 88, 64, 131, 220,
        130, 157, 3, 41, 64, 249, 137, 216, 238, 168, 117, 138, 20, 97, 103, 163, 179, 178, 124,
        243, 97, 51, 100, 124, 167, 155, 211, 3, 12, 97, 131, 49, 105, 220, 215, 77, 32, 153, 212,
        242, 32, 78, 32, 149, 145, 193, 95, 189, 160, 188, 48, 143, 144, 94, 204, 192, 81, 223, 57,
        166, 170, 105, 88, 64, 91, 129, 79, 227, 172, 143, 222, 59, 23, 24, 146, 41, 147, 17, 155,
        22, 163, 49, 137, 57, 217, 93, 103, 251, 223, 172, 89, 98, 49, 135, 7, 172, 130, 222, 196,
        121, 171, 26, 110, 156, 159, 167, 17, 19, 36, 17, 99, 37, 193, 78, 152, 24, 114, 86, 30,
        16, 186, 227, 2, 225, 1, 141, 121, 2, 255, 131, 2, 160, 217, 1, 2, 128, 159, 255, 130, 128,
        159, 255, 129, 160,
    ];

    // a block with an update payload and vote
    const HEADER_HASH2: &str = "6da1c6dffaa21dd72034dae5fcafb1dea8dc0ff9d246910f76e8f8a91fc8fe4c";
    const BLOCK2: &'static [u8] = &[
        130, 1, 131, 133, 26, 37, 192, 15, 169, 88, 32, 159, 185, 213, 249, 53, 208, 58, 143, 109,
        225, 163, 182, 41, 57, 245, 132, 195, 90, 16, 43, 106, 178, 38, 184, 22, 50, 117, 191, 190,
        8, 155, 209, 132, 131, 0, 88, 32, 14, 87, 81, 192, 38, 229, 67, 178, 232, 171, 46, 176, 96,
        153, 218, 161, 209, 229, 223, 71, 119, 143, 119, 135, 250, 171, 69, 205, 241, 47, 227, 168,
        88, 32, 175, 192, 218, 100, 24, 59, 242, 102, 79, 61, 78, 236, 114, 56, 213, 36, 186, 96,
        127, 174, 234, 178, 79, 193, 0, 235, 134, 29, 186, 105, 151, 27, 131, 2, 88, 32, 211, 106,
        38, 25, 166, 114, 73, 70, 4, 225, 27, 180, 71, 203, 207, 82, 49, 233, 242, 186, 37, 194,
        22, 145, 119, 237, 201, 65, 189, 80, 173, 108, 88, 32, 211, 106, 38, 25, 166, 114, 73, 70,
        4, 225, 27, 180, 71, 203, 207, 82, 49, 233, 242, 186, 37, 194, 22, 145, 119, 237, 201, 65,
        189, 80, 173, 108, 88, 32, 175, 192, 218, 100, 24, 59, 242, 102, 79, 61, 78, 236, 114, 56,
        213, 36, 186, 96, 127, 174, 234, 178, 79, 193, 0, 235, 134, 29, 186, 105, 151, 27, 88, 32,
        140, 158, 40, 166, 228, 7, 153, 96, 7, 252, 152, 158, 122, 142, 177, 242, 63, 166, 96, 141,
        252, 74, 49, 25, 173, 189, 198, 247, 182, 94, 86, 206, 132, 130, 1, 25, 83, 34, 88, 64,
        144, 150, 195, 151, 47, 188, 91, 146, 61, 0, 85, 139, 174, 226, 125, 64, 190, 65, 108, 155,
        218, 146, 21, 17, 224, 106, 161, 180, 28, 56, 135, 111, 38, 211, 143, 171, 209, 191, 173,
        171, 36, 167, 36, 183, 140, 0, 118, 215, 244, 245, 28, 231, 104, 250, 65, 158, 23, 196, 64,
        46, 81, 60, 146, 183, 129, 25, 166, 167, 130, 2, 130, 132, 0, 88, 64, 144, 150, 195, 151,
        47, 188, 91, 146, 61, 0, 85, 139, 174, 226, 125, 64, 190, 65, 108, 155, 218, 146, 21, 17,
        224, 106, 161, 180, 28, 56, 135, 111, 38, 211, 143, 171, 209, 191, 173, 171, 36, 167, 36,
        183, 140, 0, 118, 215, 244, 245, 28, 231, 104, 250, 65, 158, 23, 196, 64, 46, 81, 60, 146,
        183, 88, 64, 212, 46, 47, 25, 216, 7, 138, 94, 125, 130, 233, 29, 241, 131, 236, 170, 213,
        118, 166, 7, 218, 148, 190, 35, 238, 152, 86, 184, 166, 132, 138, 194, 228, 9, 231, 66,
        186, 193, 173, 106, 201, 14, 194, 77, 31, 146, 33, 32, 83, 201, 21, 115, 84, 197, 42, 58,
        177, 76, 68, 24, 116, 77, 195, 164, 88, 64, 183, 39, 181, 143, 182, 129, 87, 177, 29, 188,
        198, 114, 67, 36, 110, 135, 35, 123, 187, 26, 177, 69, 19, 3, 60, 227, 41, 86, 90, 140, 49,
        186, 226, 251, 61, 205, 67, 178, 168, 63, 226, 40, 0, 252, 148, 41, 65, 78, 121, 161, 200,
        28, 87, 68, 194, 124, 33, 231, 41, 198, 211, 103, 167, 15, 88, 64, 186, 117, 115, 53, 139,
        41, 75, 202, 58, 51, 93, 145, 4, 51, 218, 82, 147, 70, 22, 96, 248, 49, 152, 98, 153, 136,
        22, 42, 134, 186, 81, 210, 20, 141, 29, 141, 115, 141, 120, 100, 208, 184, 6, 9, 83, 3,
        228, 247, 145, 150, 123, 18, 160, 100, 240, 159, 121, 179, 94, 16, 101, 236, 115, 4, 132,
        131, 0, 0, 0, 130, 106, 99, 97, 114, 100, 97, 110, 111, 45, 115, 108, 1, 160, 88, 32, 75,
        169, 42, 163, 32, 198, 10, 204, 154, 215, 185, 166, 79, 46, 218, 85, 196, 210, 236, 40,
        230, 4, 250, 241, 134, 112, 139, 79, 12, 78, 142, 223, 132, 159, 255, 131, 2, 160, 217, 1,
        2, 128, 159, 255, 130, 129, 135, 131, 0, 0, 0, 142, 129, 0, 129, 25, 78, 32, 129, 26, 0,
        30, 132, 128, 129, 26, 0, 30, 132, 128, 129, 25, 16, 0, 129, 25, 2, 188, 129, 27, 0, 0, 18,
        48, 156, 229, 64, 0, 129, 27, 0, 0, 0, 69, 217, 100, 184, 0, 129, 27, 0, 0, 0, 232, 212,
        165, 16, 0, 129, 27, 0, 0, 90, 243, 16, 122, 64, 0, 129, 25, 39, 16, 128, 128, 128, 130,
        108, 99, 115, 108, 45, 100, 97, 101, 100, 97, 108, 117, 115, 1, 162, 101, 109, 97, 99, 111,
        115, 132, 88, 32, 3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98,
        177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20, 88, 32, 223, 77, 88,
        230, 96, 122, 72, 37, 229, 166, 244, 42, 151, 158, 142, 198, 132, 56, 107, 185, 126, 148,
        61, 53, 92, 64, 155, 11, 210, 230, 211, 240, 88, 32, 3, 23, 10, 46, 117, 151, 183, 183,
        227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207,
        76, 17, 19, 20, 88, 32, 3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29, 19,
        154, 98, 177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20, 101, 119,
        105, 110, 54, 52, 132, 88, 32, 3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57, 29,
        19, 154, 98, 177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20, 88, 32,
        168, 165, 253, 158, 16, 211, 205, 199, 174, 69, 91, 18, 57, 101, 123, 89, 14, 248, 166,
        131, 155, 35, 138, 194, 58, 150, 19, 150, 35, 9, 129, 130, 88, 32, 3, 23, 10, 46, 117, 151,
        183, 183, 227, 216, 76, 5, 57, 29, 19, 154, 98, 177, 87, 231, 135, 134, 216, 192, 130, 242,
        157, 207, 76, 17, 19, 20, 88, 32, 3, 23, 10, 46, 117, 151, 183, 183, 227, 216, 76, 5, 57,
        29, 19, 154, 98, 177, 87, 231, 135, 134, 216, 192, 130, 242, 157, 207, 76, 17, 19, 20, 160,
        88, 64, 177, 206, 41, 107, 90, 64, 174, 37, 206, 236, 140, 204, 164, 28, 138, 163, 135, 6,
        247, 65, 194, 20, 62, 58, 228, 134, 157, 219, 206, 46, 13, 140, 244, 233, 209, 194, 168,
        28, 12, 29, 27, 164, 69, 228, 61, 211, 228, 192, 254, 244, 41, 143, 130, 245, 96, 214, 139,
        28, 142, 195, 75, 70, 118, 86, 88, 64, 229, 155, 235, 248, 100, 67, 74, 242, 134, 202, 249,
        219, 152, 6, 179, 253, 209, 138, 196, 174, 105, 105, 64, 145, 28, 140, 188, 108, 56, 209,
        34, 212, 73, 199, 91, 59, 17, 37, 51, 64, 7, 149, 223, 189, 115, 208, 159, 160, 92, 59,
        208, 211, 132, 232, 103, 194, 111, 235, 22, 89, 55, 76, 172, 10, 159, 132, 88, 64, 177,
        206, 41, 107, 90, 64, 174, 37, 206, 236, 140, 204, 164, 28, 138, 163, 135, 6, 247, 65, 194,
        20, 62, 58, 228, 134, 157, 219, 206, 46, 13, 140, 244, 233, 209, 194, 168, 28, 12, 29, 27,
        164, 69, 228, 61, 211, 228, 192, 254, 244, 41, 143, 130, 245, 96, 214, 139, 28, 142, 195,
        75, 70, 118, 86, 88, 32, 78, 228, 14, 59, 127, 135, 202, 220, 100, 241, 24, 251, 168, 246,
        117, 236, 6, 39, 244, 171, 247, 62, 96, 155, 54, 166, 51, 159, 23, 132, 119, 175, 245, 88,
        64, 23, 51, 112, 47, 69, 167, 162, 156, 160, 154, 7, 137, 47, 202, 5, 28, 159, 73, 142,
        153, 66, 99, 19, 126, 31, 13, 77, 247, 204, 231, 255, 156, 72, 22, 93, 93, 222, 106, 199,
        234, 64, 78, 194, 232, 242, 152, 97, 120, 133, 148, 69, 139, 111, 193, 42, 167, 181, 8, 45,
        166, 223, 190, 90, 13, 255, 129, 160,
    ];

    // a block with a VSS certificate
    const HEADER_HASH3: &str = "4c64f52a24d01ac5f66c4d23eec2e009c9e81d54279b5f2b6aaedf75d3ee7047";
    const BLOCK3: &'static [u8] = &[
        130, 1, 131, 133, 26, 37, 192, 15, 169, 88, 32, 209, 226, 244, 50, 10, 63, 168, 77, 148,
        141, 125, 21, 56, 67, 105, 17, 139, 229, 234, 17, 76, 161, 243, 88, 238, 246, 102, 241,
        171, 227, 216, 116, 132, 131, 0, 88, 32, 14, 87, 81, 192, 38, 229, 67, 178, 232, 171, 46,
        176, 96, 153, 218, 161, 209, 229, 223, 71, 119, 143, 119, 135, 250, 171, 69, 205, 241, 47,
        227, 168, 88, 32, 175, 192, 218, 100, 24, 59, 242, 102, 79, 61, 78, 236, 114, 56, 213, 36,
        186, 96, 127, 174, 234, 178, 79, 193, 0, 235, 134, 29, 186, 105, 151, 27, 131, 0, 88, 32,
        37, 119, 122, 202, 158, 74, 115, 212, 143, 199, 59, 79, 150, 29, 52, 91, 6, 212, 166, 243,
        73, 203, 121, 22, 87, 13, 53, 83, 125, 83, 71, 159, 88, 32, 171, 50, 150, 121, 155, 118,
        62, 64, 134, 56, 209, 229, 206, 66, 25, 13, 165, 127, 82, 155, 4, 98, 196, 75, 36, 173, 40,
        30, 96, 170, 51, 12, 88, 32, 175, 192, 218, 100, 24, 59, 242, 102, 79, 61, 78, 236, 114,
        56, 213, 36, 186, 96, 127, 174, 234, 178, 79, 193, 0, 235, 134, 29, 186, 105, 151, 27, 88,
        32, 78, 102, 40, 12, 217, 77, 89, 16, 114, 52, 155, 236, 10, 48, 144, 165, 58, 169, 69, 86,
        46, 251, 109, 8, 213, 110, 83, 101, 75, 14, 64, 152, 132, 130, 6, 0, 88, 64, 104, 223, 175,
        89, 1, 23, 220, 41, 221, 163, 18, 217, 147, 123, 103, 73, 115, 219, 220, 166, 12, 151, 134,
        57, 22, 175, 249, 85, 126, 231, 112, 193, 199, 165, 231, 106, 53, 214, 31, 86, 240, 239,
        173, 238, 9, 126, 41, 80, 80, 90, 97, 225, 121, 209, 238, 116, 193, 207, 122, 3, 216, 139,
        62, 171, 129, 26, 0, 1, 248, 142, 130, 2, 130, 132, 0, 88, 64, 104, 223, 175, 89, 1, 23,
        220, 41, 221, 163, 18, 217, 147, 123, 103, 73, 115, 219, 220, 166, 12, 151, 134, 57, 22,
        175, 249, 85, 126, 231, 112, 193, 199, 165, 231, 106, 53, 214, 31, 86, 240, 239, 173, 238,
        9, 126, 41, 80, 80, 90, 97, 225, 121, 209, 238, 116, 193, 207, 122, 3, 216, 139, 62, 171,
        88, 64, 177, 206, 41, 107, 90, 64, 174, 37, 206, 236, 140, 204, 164, 28, 138, 163, 135, 6,
        247, 65, 194, 20, 62, 58, 228, 134, 157, 219, 206, 46, 13, 140, 244, 233, 209, 194, 168,
        28, 12, 29, 27, 164, 69, 228, 61, 211, 228, 192, 254, 244, 41, 143, 130, 245, 96, 214, 139,
        28, 142, 195, 75, 70, 118, 86, 88, 64, 36, 124, 229, 251, 42, 159, 123, 168, 46, 160, 132,
        99, 26, 102, 166, 21, 30, 18, 221, 162, 211, 200, 133, 231, 228, 28, 67, 208, 198, 205,
        249, 77, 151, 192, 146, 77, 149, 202, 35, 108, 53, 199, 191, 224, 135, 76, 205, 124, 154,
        117, 154, 196, 109, 50, 150, 157, 234, 4, 35, 69, 215, 184, 133, 3, 88, 64, 57, 70, 179,
        234, 200, 48, 162, 65, 118, 245, 63, 246, 0, 186, 127, 101, 198, 17, 39, 149, 100, 65, 73,
        167, 49, 156, 146, 255, 124, 29, 113, 50, 56, 50, 76, 134, 133, 182, 30, 165, 77, 34, 116,
        181, 131, 119, 127, 21, 205, 226, 24, 251, 164, 140, 112, 68, 88, 62, 39, 43, 198, 191, 80,
        3, 132, 131, 0, 0, 0, 130, 106, 99, 97, 114, 100, 97, 110, 111, 45, 115, 108, 1, 160, 88,
        32, 75, 169, 42, 163, 32, 198, 10, 204, 154, 215, 185, 166, 79, 46, 218, 85, 196, 210, 236,
        40, 230, 4, 250, 241, 134, 112, 139, 79, 12, 78, 142, 223, 132, 159, 255, 131, 0, 217, 1,
        2, 128, 217, 1, 2, 129, 132, 88, 35, 88, 33, 3, 196, 137, 30, 236, 154, 198, 148, 194, 223,
        219, 145, 32, 251, 86, 79, 250, 153, 252, 154, 164, 67, 210, 121, 93, 140, 31, 241, 216,
        168, 51, 96, 197, 11, 88, 64, 40, 177, 28, 201, 177, 199, 236, 55, 223, 215, 186, 168, 115,
        163, 72, 5, 234, 15, 107, 120, 178, 25, 180, 21, 83, 52, 27, 128, 74, 242, 189, 215, 143,
        7, 221, 71, 241, 196, 198, 225, 58, 31, 249, 41, 129, 165, 248, 28, 114, 141, 77, 216, 39,
        166, 148, 104, 67, 96, 105, 177, 115, 129, 144, 1, 88, 64, 246, 124, 178, 150, 119, 65,
        215, 29, 37, 137, 39, 103, 207, 66, 225, 166, 139, 159, 188, 234, 68, 185, 157, 99, 36,
        183, 43, 13, 150, 44, 164, 89, 63, 154, 61, 116, 18, 69, 153, 80, 40, 191, 193, 248, 146,
        79, 72, 91, 122, 233, 202, 167, 105, 97, 199, 225, 219, 26, 147, 110, 50, 81, 47, 16, 159,
        255, 130, 128, 159, 255, 129, 160,
    ];
}
