extern crate cardano;
extern crate exe_common;
extern crate rand;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use cardano::{
    address, block, coin, config, fee,
    hdwallet::{self, Seed, XPrv},
};
use rand::{thread_rng, Rng};
use std::collections::BTreeMap;
use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, SystemTime};

fn generate_key() -> XPrv {
    let mut seed = [0u8; hdwallet::SEED_SIZE];
    thread_rng().fill(&mut seed[..]);
    let seed = Seed::from_bytes(seed);
    XPrv::generate_from_seed(&seed)
}

fn write_file(path: &Path, s: &String) {
    let mut file = File::create(path).unwrap();
    file.write_all(s.as_bytes()).unwrap();
}

#[derive(Serialize)]
struct BootAddress {
    xprv: String,
    addr: String,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let dest_dir = Path::new(&args[1]);
    let nr_nodes = args[2].parse::<usize>().unwrap();
    let nr_addresses = args[3].parse::<usize>().unwrap();

    let protocol_magic = 328429219.into();

    let mut boot_stakeholders = BTreeMap::new();

    for n in 0..nr_nodes {
        let stakeholder_prv = generate_key();
        let stakeholder_pk = stakeholder_prv.public();
        let delegate_prv = generate_key();
        let delegate_pk = delegate_prv.public();

        write_file(
            &dest_dir.join(format!("stakeholder-{}.xprv", n)),
            &stakeholder_prv.to_string(),
        );
        write_file(
            &dest_dir.join(format!("delegate-{}.xprv", n)),
            &delegate_prv.to_string(),
        );

        let stakeholder_id = address::StakeholderId::new(&stakeholder_pk);

        let psk =
            block::sign::ProxySecretKey::sign(&stakeholder_prv, delegate_pk, 0, protocol_magic);

        boot_stakeholders.insert(
            stakeholder_id,
            config::BootStakeholder {
                weight: 1,
                issuer_pk: psk.issuer_pk,
                delegate_pk: psk.delegate_pk,
                cert: psk.cert,
            },
        );
    }

    let mut non_avvm_balances = BTreeMap::new();
    let mut boot_addresses = vec![];

    for _ in 0..nr_addresses {
        // FIXME: generate from HD wallets?
        let addr_prv = generate_key();
        let addr: address::Addr =
            address::ExtendedAddr::new_simple(addr_prv.public(), protocol_magic.into()).into();
        boot_addresses.push(BootAddress {
            xprv: addr_prv.to_string(),
            addr: addr.to_string(),
        });
        non_avvm_balances.insert(addr, coin::Coin::new(19999999999999).unwrap());
    }

    let genesis_data = config::GenesisData {
        genesis_prev: cardano::block::HeaderHash::new(&[0; cardano::hash::Blake2b256::HASH_SIZE]),
        epoch_stability_depth: 2160,
        start_time: SystemTime::UNIX_EPOCH + Duration::from_secs(1548089245),
        slot_duration: Duration::from_millis(20000),
        protocol_magic,
        fee_policy: fee::LinearFee::new(
            fee::Milli::new(43, 946),
            fee::Milli::integral(155381),
        ),
        avvm_distr: BTreeMap::new(),
        non_avvm_balances,
        boot_stakeholders,
    };

    let (genesis_data, genesis_hash) = exe_common::genesisdata::print::print(genesis_data).unwrap();

    eprintln!("Genesis hash = {}", genesis_hash);

    write_file(&dest_dir.join("genesis.json"), &genesis_data);

    write_file(
        &dest_dir.join("addresses.json"),
        &serde_json::to_string_pretty(&boot_addresses).unwrap(),
    );

    write_file(&dest_dir.join("genesis.hash"), &genesis_hash.to_string());
}
