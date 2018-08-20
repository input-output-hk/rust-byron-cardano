extern crate protocol;
extern crate cardano;
extern crate rand;
#[macro_use]
extern crate log;
extern crate env_logger;

use protocol::packet::{Handshake};

use protocol::command::{Command};

use protocol::{command, ntt, Connection};
use cardano::{config::ProtocolMagic, hdwallet,
              wallet::scheme::{Wallet, Account},
              wallet::bip44,
              fee, txutils, tx, coin, util::{base58, hex}};
use std::net::TcpStream;
use std::fs::File;
use std::io::prelude::*;

// mainnet:
// const HOST: &'static str = "relays.cardano-mainnet.iohk.io:3000";
// const PROTOCOL_MAGIC : u32 = 764824073;

// staging:
//const HOST: &'static str = "relays.awstest.iohkdev.io:3000";
const HOST: &'static str = "localhost:3000";
const PROTOCOL_MAGIC : u32 = 633343913;

fn main() {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Trace)
        .init();

    info!("############## starting test");
    let drg_seed = rand::random();
    let mut hs = Handshake::default();
    hs.protocol_magic = ProtocolMagic::new(PROTOCOL_MAGIC);

    info!("############## connecting to {}", HOST);
    let stream = TcpStream::connect(HOST).unwrap();
    stream.set_nodelay(true).unwrap();

    info!("############## sending handshake to {}", HOST);
    let conn = ntt::Connection::handshake(drg_seed, stream).unwrap();
    let mut connection = Connection::new(conn);
    connection.handshake(&hs).unwrap();

    if false {
        let mbh = command::GetBlockHeader::tip().execute(&mut connection)
            .expect("to get one header at least").decode().unwrap();
        info!("tip date: {}", mbh[0].get_blockdate());
    }

    {
        let mut file = File::open("test.key").expect("unable to read test.key");
        let mut key = String::new();
        file.read_to_string(&mut key).unwrap();

        // 1. create a wallet
        let xprv_vec = hex::decode(&key).unwrap();
        assert!(xprv_vec.len() == hdwallet::XPRV_SIZE);
        let mut xprv_bytes = [0;hdwallet::XPRV_SIZE];
        xprv_bytes.copy_from_slice(&xprv_vec[..]);
        let root_xprv =
            hdwallet::XPrv::from_bytes_verified(xprv_bytes).unwrap();
        let mut wallet = bip44::Wallet::from_cached_key(
            bip44::RootLevel::from(root_xprv), hdwallet::DerivationScheme::default());
        let account_number = 0;
        let account = wallet.create_account("bla", account_number);

        // 2. create a valid transaction
        let input_index = 2;
        let input_addr = account.generate_addresses(
            [(bip44::AddrType::External, input_index)].iter()).pop().unwrap();
        let output_addr = account.generate_addresses(
            [(bip44::AddrType::External, input_index + 1)].iter()).pop().unwrap();
        let change_addr = account.generate_addresses(
            [(bip44::AddrType::Internal, 1)].iter()).pop().unwrap();

        let txin = tx::TxIn::new(tx::TxId::from_slice(&hex::decode("e276efdd613403ed096471c361b78f53b942de3904fbb142e838069e4374a793").unwrap()).unwrap(), 0);
        let addressing = bip44::Addressing::new(account_number, bip44::AddrType::External, input_index).unwrap();
        let txout = tx::TxOut::new(input_addr.clone(), coin::Coin::new(600_000).unwrap());
        let inputs = vec![txutils::Input::new(txin, txout, addressing)];

        let outputs = vec![tx::TxOut::new(output_addr.clone(), coin::Coin::new(400_000).unwrap())];

        let (txaux, fee) = wallet.new_transaction(
            ProtocolMagic::new(PROTOCOL_MAGIC),
            fee::SelectionPolicy::default(),
            inputs.iter(),
            outputs,
            &txutils::OutputPolicy::One(change_addr.clone())).unwrap();

        info!("############## transaction prepared");
        info!("  txaux {:?}", txaux);
        info!("  tx id {}", txaux.tx.id());
        info!("  from address {}", base58::encode(&input_addr.to_bytes()));
        info!("  to address {}", base58::encode(&output_addr.to_bytes()));
        info!("  change to address {}", base58::encode(&change_addr.to_bytes()));
        info!("  fee: {:?}", fee);

        // 3. send the transaction
        info!(" == Anounce New Tx ====================================");
        command::SendTx::new(txaux).execute(&mut connection)
            .expect("send new tx");
    }
}
