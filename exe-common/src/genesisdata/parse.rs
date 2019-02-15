use base64;
use cardano::{address, block, coin, config, fee, hdwallet, redeem};
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

    let protocol_magic = config::ProtocolMagic::from(data.protocolConsts.protocolMagic);

    let parse_fee_constant = |s: &str| {
        let n = s.parse::<u64>().unwrap();
        assert!(n % 1000000 == 0);
        fee::Milli::new(n / 1000000000, n / 1000000 % 1000)
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

    let mut boot_stakeholders = BTreeMap::new();

    for (stakeholder_id, weight) in &data.bootStakeholders {
        let heavy = data.heavyDelegation.get(stakeholder_id).unwrap();

        let stakeholder_id = address::StakeholderId::from_str(stakeholder_id).unwrap();

        let psk = cardano::block::sign::ProxySecretKey {
            omega: 0,
            issuer_pk: hdwallet::XPub::from_slice(&base64::decode(&heavy.issuerPk).unwrap())
                .unwrap(),
            delegate_pk: hdwallet::XPub::from_slice(&base64::decode(&heavy.delegatePk).unwrap())
                .unwrap(),
            cert: hdwallet::Signature::<()>::from_hex(&heavy.cert).unwrap(),
        };

        // Check that the stakeholder ID corresponds to the issuer public key.
        assert_eq!(stakeholder_id, address::StakeholderId::new(&psk.issuer_pk));

        // Check that the certificate is correct.
        assert!(psk.verify(protocol_magic));

        boot_stakeholders.insert(
            stakeholder_id,
            config::BootStakeholder {
                weight: *weight,
                issuer_pk: psk.issuer_pk,
                delegate_pk: psk.delegate_pk,
                cert: psk.cert,
            },
        );
    }

    config::GenesisData {
        genesis_prev,
        epoch_stability_depth: data.protocolConsts.k,
        protocol_magic,
        fee_policy: fee::LinearFee::new(
            parse_fee_constant(&data.blockVersionData.txFeePolicy.summand),
            parse_fee_constant(&data.blockVersionData.txFeePolicy.multiplier),
        ),
        avvm_distr,
        non_avvm_balances,
        start_time,
        slot_duration,
        boot_stakeholders,
    }
}

pub fn canonicalize_json<R: Read>(json: R) -> String {
    let data: serde_json::Value = serde_json::from_reader(json).unwrap();
    data.to_string()
}

#[cfg(test)]
mod test {

    use super::*;
    use cardano::{coin, fee::Milli};

    #[test]
    pub fn test() {
        let genesis_hash = cardano::block::HeaderHash::from_str(
            &"c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323",
        )
        .unwrap();

        let genesis_data = super::parse(
            super::super::data::get_genesis_data(&genesis_hash)
                .unwrap()
                .as_bytes(),
        );

        assert_eq!(genesis_data.epoch_stability_depth, 2160);
        assert_eq!(
            genesis_data
                .start_time
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            1506450213
        );
        assert_eq!(genesis_data.slot_duration.as_secs(), 20);
        assert_eq!(genesis_data.slot_duration.subsec_millis(), 0);
        assert_eq!(genesis_data.protocol_magic, 633343913.into());
        assert_eq!(genesis_data.fee_policy.coefficient, Milli::new(43, 946));
        assert_eq!(genesis_data.fee_policy.constant, Milli::integral(155381));

        assert_eq!(
            base64::encode_config(
                genesis_data
                    .avvm_distr
                    .iter()
                    .find(|(_, v)| **v == coin::Coin::new(9999300000000).unwrap())
                    .unwrap()
                    .0,
                base64::URL_SAFE
            ),
            "-0BJDi-gauylk4LptQTgjMeo7kY9lTCbZv12vwOSTZk="
        );

        let genesis_hash = cardano::block::HeaderHash::from_str(
            &"b7f76950bc4866423538ab7764fc1c7020b24a5f717a5bee3109ff2796567214",
        )
        .unwrap();

        let genesis_data = super::parse(
            super::super::data::get_genesis_data(&genesis_hash)
                .unwrap()
                .as_bytes(),
        );

        assert_eq!(
            genesis_data
                .non_avvm_balances
                .iter()
                .find(|(n, _)| n.to_string()
                    == "2cWKMJemoBaheSTiK9XEtQDf47Z3My8jwN25o5jjm7s7jaXin2nothhWQrTDd8m433M8K")
                .unwrap()
                .1,
            &coin::Coin::new(5428571428571429).unwrap()
        );
    }

}
