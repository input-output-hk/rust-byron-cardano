use cardano::{block::HeaderHash, config};
use genesisdata::{parse, raw};
use std::time::SystemTime;

/// Return a canonical JSON representation of the given genesis data,
/// as well as the corresponding genesis hash.
pub fn print(
    mut genesis_data: config::GenesisData,
) -> Result<(String, HeaderHash), std::io::Error> {
    let raw = raw::GenesisData {
        avvmDistr: genesis_data
            .avvm_distr
            .iter()
            .map(|(avvm, balance)| {
                (
                    base64::encode_config(avvm, base64::URL_SAFE),
                    u64::from(*balance).to_string(),
                )
            })
            .collect(),
        nonAvvmBalances: genesis_data
            .non_avvm_balances
            .iter()
            .map(|(address, balance)| (address.to_string(), u64::from(*balance).to_string()))
            .collect(),
        bootStakeholders: genesis_data
            .boot_stakeholders
            .iter()
            .map(|(stakeholder_id, stakeholder)| (stakeholder_id.to_string(), stakeholder.weight))
            .collect(),
        heavyDelegation: genesis_data
            .boot_stakeholders
            .iter()
            .map(|(stakeholder_id, stakeholder)| {
                (
                    stakeholder_id.to_string(),
                    raw::HeavyDelegation {
                        issuerPk: base64::encode(&stakeholder.issuer_pk),
                        delegatePk: base64::encode(&stakeholder.delegate_pk),
                        cert: stakeholder.cert.to_string(),
                    },
                )
            })
            .collect(),
        protocolConsts: raw::ProtocolConsts {
            k: genesis_data.epoch_stability_depth,
            protocolMagic: genesis_data.protocol_magic.into(),
        },
        startTime: genesis_data
            .start_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        blockVersionData: raw::BlockVersionData {
            slotDuration: (genesis_data.slot_duration.as_secs() as u64 * 1000
                + genesis_data.slot_duration.subsec_millis() as u64)
                .to_string(),
            txFeePolicy: raw::TxFeePolicy {
                summand: (genesis_data.fee_policy.constant.as_millis() * 1000000).to_string(),
                multiplier: (genesis_data.fee_policy.coefficient.as_millis() * 1000000).to_string(),
            },
        },
    };

    let json = serde_json::to_string(&raw)?;

    // Compute the hash over the canonical JSON.
    let canon_json = parse::canonicalize_json(json.as_bytes());
    let genesis_hash = HeaderHash::new(canon_json.as_bytes());

    genesis_data.genesis_prev = genesis_hash.clone(); // ugly
    assert_eq!(genesis_data, parse::parse(json.as_bytes()));

    Ok((canon_json, genesis_hash))
}
