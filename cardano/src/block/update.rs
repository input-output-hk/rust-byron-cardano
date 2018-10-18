use super::types;
use cbor_event::{self, de::Deserializer, se::Serializer};
use hash::{self, Blake2b256};
use hdwallet;

use std::{
    collections::BTreeMap,
    fmt,
    io::{BufRead, Write},
};

#[derive(Debug, Clone)]
pub struct UpdatePayload {
    pub proposal: Option<UpdateProposal>,
    pub votes: Vec<UpdateVote>,
}

impl cbor_event::se::Serialize for UpdatePayload {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let serializer = serializer
            .write_array(cbor_event::Len::Len(2))?
            .serialize(&self.proposal)?;
        cbor_event::se::serialize_indefinite_array(self.votes.iter(), serializer)
    }
}

impl cbor_event::de::Deserialize for UpdatePayload {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(2, "UpdatePayload")?;
        Ok(Self {
            proposal: raw.deserialize()?,
            votes: raw.deserialize()?,
        })
    }
}

/// Witness of delegation payload consisting of a simple hash
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateProof(Blake2b256);

impl UpdateProof {
    pub fn generate(update: &UpdatePayload) -> Self {
        let h = Blake2b256::new(&cbor!(update).unwrap());
        UpdateProof(h)
    }
}

impl fmt::Display for UpdateProof {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl cbor_event::se::Serialize for UpdateProof {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.serialize(&self.0)
    }
}

impl cbor_event::de::Deserialize for UpdateProof {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        let hash = cbor_event::de::Deserialize::deserialize(raw)?;
        Ok(UpdateProof(hash))
    }
}

#[derive(Debug, Clone)]
pub struct UpdateProposal {
    pub block_version: types::BlockVersion,
    pub block_version_mod: BlockVersionModifier,
    pub software_version: types::SoftwareVersion,
    pub data: BTreeMap<SystemTag, UpdateData>,
    pub attributes: UpAttributes,
    pub from: hdwallet::XPub,
    pub signature: hdwallet::Signature<()>, // UpdateProposalToSign
}

pub type UpAttributes = types::Attributes;
pub type SystemTag = String;

impl cbor_event::se::Serialize for UpdateProposal {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let serializer = serializer
            .write_array(cbor_event::Len::Len(7))?
            .serialize(&self.block_version)?
            .serialize(&self.block_version_mod)?
            .serialize(&self.software_version)?;
        cbor_event::se::serialize_fixed_map(self.data.iter(), serializer)?
            .serialize(&self.attributes)?
            .serialize(&self.from)?
            .serialize(&self.signature)
    }
}

impl cbor_event::de::Deserialize for UpdateProposal {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(7, "UpdateProposal")?;
        Ok(Self {
            block_version: raw.deserialize()?,
            block_version_mod: raw.deserialize()?,
            software_version: raw.deserialize()?,
            data: raw.deserialize()?,
            attributes: raw.deserialize()?,
            from: raw.deserialize()?,
            signature: raw.deserialize()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct BlockVersionModifier {
    pub script_version: Option<ScriptVersion>,
    pub slot_duration: Option<Millisecond>,
    pub max_block_size: Option<u64>,
    pub max_header_size: Option<u64>,
    pub max_tx_size: Option<u64>,
    pub max_proposal_size: Option<u64>,
    pub mpc_thd: Option<types::CoinPortion>,
    pub heavy_del_thd: Option<types::CoinPortion>,
    pub update_vote_thd: Option<types::CoinPortion>,
    pub update_proposal_thd: Option<types::CoinPortion>,
    pub update_implicit: Option<FlatSlotId>,
    pub softfork_rule: Option<SoftforkRule>,
    pub tx_fee_policy: Option<TxFeePolicy>,
    pub unlock_stake_epoch: Option<types::EpochId>,
}

impl cbor_event::se::Serialize for BlockVersionModifier {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        assert!(self.tx_fee_policy.is_none()); // not tested yet
        serializer
            .write_array(cbor_event::Len::Len(14))?
            .serialize(&self.script_version)?
            .serialize(&self.slot_duration)?
            .serialize(&self.max_block_size)?
            .serialize(&self.max_header_size)?
            .serialize(&self.max_tx_size)?
            .serialize(&self.max_proposal_size)?
            .serialize(&self.mpc_thd)?
            .serialize(&self.heavy_del_thd)?
            .serialize(&self.update_vote_thd)?
            .serialize(&self.update_proposal_thd)?
            .serialize(&self.update_implicit)?
            .serialize(&self.softfork_rule)?
            .serialize(&self.tx_fee_policy)?
            .serialize(&self.unlock_stake_epoch)
    }
}

impl cbor_event::de::Deserialize for BlockVersionModifier {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(14, "BlockVersionModifier")?;
        Ok(Self {
            script_version: raw.deserialize()?,
            slot_duration: raw.deserialize()?,
            max_block_size: raw.deserialize()?,
            max_header_size: raw.deserialize()?,
            max_tx_size: raw.deserialize()?,
            max_proposal_size: raw.deserialize()?,
            mpc_thd: raw.deserialize()?,
            heavy_del_thd: raw.deserialize()?,
            update_vote_thd: raw.deserialize()?,
            update_proposal_thd: raw.deserialize()?,
            update_implicit: raw.deserialize()?,
            softfork_rule: raw.deserialize()?,
            tx_fee_policy: raw.deserialize()?,
            unlock_stake_epoch: raw.deserialize()?,
        })
    }
}

pub type ScriptVersion = u16;
pub type Millisecond = u64;
pub type FlatSlotId = u64;
pub type TxFeePolicy = cbor_event::Value; // TODO

#[derive(Debug, Clone)]
pub struct UpdateData {
    pub app_diff_hash: hash::Blake2b256,
    pub pkg_hash: hash::Blake2b256,
    pub updater_hash: hash::Blake2b256,
    pub metadata_hash: hash::Blake2b256,
}

impl cbor_event::se::Serialize for UpdateData {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(4))?
            .serialize(&self.app_diff_hash)?
            .serialize(&self.pkg_hash)?
            .serialize(&self.updater_hash)?
            .serialize(&self.metadata_hash)
    }
}

impl cbor_event::de::Deserialize for UpdateData {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(4, "UpdateData")?;
        Ok(Self {
            app_diff_hash: raw.deserialize()?,
            pkg_hash: raw.deserialize()?,
            updater_hash: raw.deserialize()?,
            metadata_hash: raw.deserialize()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SoftforkRule {
    pub init_thd: types::CoinPortion,
    pub min_thd: types::CoinPortion,
    pub thd_decrement: types::CoinPortion,
}

impl cbor_event::se::Serialize for SoftforkRule {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer.serialize(&(&self.init_thd, &self.min_thd, &self.thd_decrement))
    }
}

impl cbor_event::de::Deserialize for SoftforkRule {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(3, "SoftforkRule")?;
        Ok(Self {
            init_thd: raw.deserialize()?,
            min_thd: raw.deserialize()?,
            thd_decrement: raw.deserialize()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct UpdateVote {
    pub key: hdwallet::XPub,
    pub proposal_id: UpId,
    pub decision: bool,
    pub signature: hdwallet::Signature<(UpId, bool)>,
}

pub type UpId = hash::Blake2b256; // UpdateProposal

impl cbor_event::se::Serialize for UpdateVote {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        serializer
            .write_array(cbor_event::Len::Len(4))?
            .serialize(&self.key)?
            .serialize(&self.proposal_id)?
            .serialize(&self.decision)?
            .serialize(&self.signature)
    }
}

impl cbor_event::de::Deserialize for UpdateVote {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        raw.tuple(4, "UpdateVote")?;
        Ok(Self {
            key: raw.deserialize()?,
            proposal_id: raw.deserialize()?,
            decision: raw.deserialize()?,
            signature: raw.deserialize()?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct UpdateProposalToSign<'a> {
    pub block_version: &'a types::BlockVersion,
    pub block_version_mod: &'a BlockVersionModifier,
    pub software_version: &'a types::SoftwareVersion,
    pub data: &'a BTreeMap<SystemTag, UpdateData>,
    pub attributes: &'a UpAttributes,
}

impl<'a> cbor_event::se::Serialize for UpdateProposalToSign<'a> {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let serializer = serializer
            .write_array(cbor_event::Len::Len(5))?
            .serialize(&self.block_version)?
            .serialize(&self.block_version_mod)?
            .serialize(&self.software_version)?;
        cbor_event::se::serialize_fixed_map(self.data.iter(), serializer)?
            .serialize(&self.attributes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hash::Blake2b256;

    #[test]
    fn debug_update_proof() {
        let h = UpdateProof(Blake2b256::new(&[0; 32]));
        assert_eq!(
            format!("{:?}", h),
            "UpdateProof(Blake2b256(0x89eb0d6a8a691dae2cd15ed0369931ce0a949ecafa5c3f93f8121833646e15c3))",
        );
    }
}
