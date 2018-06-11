extern crate protocol;
extern crate wallet_crypto;
extern crate rand;
#[macro_use]
extern crate log;
extern crate env_logger;

use protocol::packet::{Handshake};

use protocol::command::{Command};

use protocol::{command, ntt, Connection};
use wallet_crypto::{config::{ProtocolMagic, Config}, bip44, hdwallet, wallet, tx, coin, util::{base58, hex}};
use std::net::TcpStream;

use std::time::Duration;
use std::thread;


// mainnet:
// const HOST: &'static str = "relays.cardano-mainnet.iohk.io:3000";
// const PROTOCOL_MAGIC : u32 = 764824073;

// staging:
const HOST: &'static str = "relays.awstest.iohkdev.io:3000";
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

    /*
    let mbh = command::GetBlockHeader::tip().execute(&mut connection)
        .expect("to get one header at least").decode().unwrap();
    info!("tip date: {}", mbh[0].get_blockdate());
    */

    {
        // 1. create a wallet
        let cached_root_xprv = hdwallet::XPrv::from_slice(&hex::decode("xxxx").unwrap()).unwrap();
        let config = Config::new(ProtocolMagic::new(PROTOCOL_MAGIC));
        let wallet = wallet::Wallet::new(cached_root_xprv, config, Default::default());
        // 2. create a valid transaction
        let mut addresses = wallet.gen_addresses(0, bip44::AddrType::External, vec![0]).unwrap();
        let input_addr = addresses.pop().unwrap();
        let mut addresses = wallet.gen_addresses(0, bip44::AddrType::External, vec![1]).unwrap();
        let output_addr = addresses.pop().unwrap();
        let mut addresses = wallet.gen_addresses(0, bip44::AddrType::Internal, vec![1]).unwrap();
        let change_addr = addresses.pop().unwrap();

        let inputs = {
            let txin = tx::TxIn::new(tx::TxId::from_slice(&hex::decode("ba019f377600b8cacac8c9cba2c0642cb3550dcca1686b0381058bc5cffc3d18").unwrap()).unwrap(), 0);
            let addressing = bip44::Addressing::new(0, bip44::AddrType::External).unwrap();
            let txout = tx::TxOut::new(input_addr.clone(), coin::Coin::new(1_000_000).unwrap());
            let mut inputs = tx::Inputs::new();
            inputs.push(tx::Input::new(txin, txout, addressing));
            inputs
        };

        let outputs = {
            let mut outputs = tx::Outputs::new();
            outputs.push(tx::TxOut::new(output_addr.clone(), coin::Coin::new(831_051).unwrap()));
            outputs
        };

        let (txaux, fee) = wallet.new_transaction(&inputs, &outputs, &change_addr).unwrap();

        info!("############## transaction prepared");
        info!("  from address {}", base58::encode(&input_addr.to_bytes()));
        info!("  to address {}", base58::encode(&output_addr.to_bytes()));
        info!("  fee: {:?}", fee);
        // 3. send the transaction
        info!(" == Anounce New Tx ====================================");
        let sender = command::AnnounceTx::new(txaux).execute(&mut connection)
            .expect("announce new tx");

        info!(" == Send New Tx =======================================");
        let res = sender.execute(&mut connection)
            .expect("sending the transaction");
        info!("    {:?}", res);
    }
}
