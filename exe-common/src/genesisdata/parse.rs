use base64;
use cardano::{address, block, coin, config, fee, redeem};
use serde_json;
use std::collections::BTreeMap;
use std::io::Read;
use std::str::FromStr;
use std::time::{Duration, SystemTime};

use genesisdata::raw;

pub fn parse<R: Read>(json: R) -> config::GenesisData {
    // FIXME: use Result

    let data_value: serde_json::Value = serde_json::from_reader(json).unwrap();
    let genesis_prev = block::HeaderHash::new(data_value.to_string().as_bytes());
    let data: raw::GenesisData = serde_json::from_value(data_value.clone()).unwrap();

    let parse_fee_constant = |s: &str| {
        let n = s.parse::<u64>().unwrap();
        assert!(n % 1000000 == 0);
        fee::Milli(n / 1000000)
    };

    let mut avvm_distr = BTreeMap::new();
    for (avvm, balance) in &data.avvmDistr {
        avvm_distr.insert(
            redeem::PublicKey::from_slice(&base64::decode_config(avvm, base64::URL_SAFE).unwrap())
                .unwrap(),
            coin::Coin::new(balance.parse::<u64>().unwrap()).unwrap(),
        );
    }

    let slot_duration = {
        let v = data.blockVersionData.slotDuration.parse::<u64>().unwrap();
        Duration::from_millis(v)
    };

    let start_time = {
        let unix_displacement = Duration::from_secs(data.startTime);
        SystemTime::UNIX_EPOCH + unix_displacement
    };

    let mut non_avvm_balances = BTreeMap::new();
    for (address, balance) in &data.nonAvvmBalances {
        non_avvm_balances.insert(
            address::ExtendedAddr::from_str(address).unwrap().into(),
            coin::Coin::new(balance.parse::<u64>().unwrap()).unwrap(),
        );
    }

    config::GenesisData {
        genesis_prev,
        epoch_stability_depth: data.protocolConsts.k,
        protocol_magic: config::ProtocolMagic::from(data.protocolConsts.protocolMagic),
        fee_policy: fee::LinearFee::new(
            parse_fee_constant(&data.blockVersionData.txFeePolicy.summand),
            parse_fee_constant(&data.blockVersionData.txFeePolicy.multiplier),
        ),
        avvm_distr,
        non_avvm_balances,
        start_time,
        slot_duration,
    }
}

pub fn canonicalize_json<R: Read>(json: R) -> String {
    let data: serde_json::Value = serde_json::from_reader(json).unwrap();
    data.to_string()
}
