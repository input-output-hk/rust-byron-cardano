use std::collections::{HashMap, BTreeMap};
use std::str::FromStr;
use serde_json;
use cardano::{config, fee, block, coin, redeem, address, hdwallet};
use base64;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct RawGenesisData {
    avvmDistr: HashMap<String, String>,
    nonAvvmBalances: HashMap<String, String>,
    protocolConsts: ProtocolConsts,
    blockVersionData: BlockVersionData,
    bootStakeholders: HashMap<String, config::BootStakeWeight>,
    heavyDelegation: HashMap<String, HeavyDelegation>,
    //vssCerts: HashMap<String, VssCert>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct ProtocolConsts {
    k: usize,
    protocolMagic: u32,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct BlockVersionData {
    maxBlockSize: String,
    maxHeaderSize: String,
    maxTxSize: String,
    maxProposalSize: String,
    softforkRule: SoftforkRule,
    txFeePolicy: TxFeePolicy,
    updateProposalThd: String,
    updateVoteThd: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct SoftforkRule {
    initThd: String,
    minThd: String,
    thdDecrement: String,
}

#[derive(Deserialize, Debug)]
struct TxFeePolicy {
    summand: String,
    multiplier: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct HeavyDelegation {
    issuerPk: String,
    delegatePk: String,
}

pub fn parse_genesis_data(json: &str) -> config::GenesisData { // FIXME: use Result

    let data: RawGenesisData = serde_json::from_str(&json).unwrap();

    let parse_fee_constant = |s: &str| {
        let n = s.parse::<u64>().unwrap();
        assert!(n % 1000000 == 0);
        fee::Milli(n / 1000000)
    };

    let mut avvm_distr = BTreeMap::new();
    for (avvm, balance) in &data.avvmDistr {
        avvm_distr.insert(
            redeem::PublicKey::from_slice(
                &base64::decode_config(avvm, base64::URL_SAFE).unwrap()).unwrap(),
            coin::Coin::new(balance.parse::<u64>().unwrap()).unwrap());
    }

    let mut boot_stakeholders = BTreeMap::new();

    for (stakeholder_id, weight) in &data.bootStakeholders {
        let heavy = data.heavyDelegation.get(stakeholder_id).unwrap();

        let stakeholder_id = address::StakeholderId::from_str(stakeholder_id).unwrap();

        let issuer_pk = hdwallet::XPub::from_slice(
            &base64::decode(&heavy.issuerPk).unwrap()).unwrap();

        assert_eq!(stakeholder_id, address::StakeholderId::new(&issuer_pk));

        boot_stakeholders.insert(
            stakeholder_id,
            config::BootStakeholder {
                weight: *weight,
                delegate_pk: hdwallet::XPub::from_slice(
                    &base64::decode(&heavy.delegatePk).unwrap()).unwrap()
            });
    }

    config::GenesisData {
        genesis_prev: block::HeaderHash::new(canonicalize_json(json).as_bytes()),
        avvm_distr,
        non_avvm_balances: BTreeMap::new(), // FIXME
        chain_parameters: config::ChainParameters {
            protocol_magic: config::ProtocolMagic::from(data.protocolConsts.protocolMagic),
            epoch_stability_depth: data.protocolConsts.k,
            max_block_size: data.blockVersionData.maxBlockSize.parse().unwrap(),
            max_header_size: data.blockVersionData.maxHeaderSize.parse().unwrap(),
            max_tx_size: data.blockVersionData.maxTxSize.parse().unwrap(),
            max_proposal_size: data.blockVersionData.maxProposalSize.parse().unwrap(),
            softfork_init_thd: data.blockVersionData.softforkRule.initThd.parse().unwrap(),
            softfork_min_thd: data.blockVersionData.softforkRule.minThd.parse().unwrap(),
            softfork_thd_decrement: data.blockVersionData.softforkRule.thdDecrement.parse().unwrap(),
            fee_policy: fee::LinearFee::new(
                parse_fee_constant(&data.blockVersionData.txFeePolicy.summand),
                parse_fee_constant(&data.blockVersionData.txFeePolicy.multiplier)),
            update_proposal_thd: data.blockVersionData.updateProposalThd.parse().unwrap(),
            update_vote_thd: data.blockVersionData.updateVoteThd.parse().unwrap(),
        },
        boot_stakeholders
    }
}

pub fn canonicalize_json(json: &str) -> String
{
    let data: serde_json::Value = serde_json::from_str(&json).unwrap();
    data.to_string()
}
