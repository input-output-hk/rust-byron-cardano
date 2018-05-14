use wallet_crypto::{cbor, address::{ExtendedAddr}};
use wallet_crypto::util::base58;
use command::{HasCommand};
use clap::{ArgMatches, Arg, App};
use config::{Config};
use storage::{tag, pack};
use blockchain::{Block};
use exe_common::config::{net};

pub struct FindAddress;

impl HasCommand for FindAddress {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "find-addresses";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("retrieve addresses in what have been synced from the network")
            .arg(Arg::with_name("name").help("the network name").index(1).required(true))
            .arg(Arg::with_name("addresses").help("list of addresses to retrieve").multiple(true).required(true).index(2))
    }
    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        let name = value_t!(args.value_of("name"), String).unwrap();
        let mut config = Config::default();
        config.network = name;
        let netcfg_file = config.get_storage_config().get_config_file();
        let net_cfg = net::Config::from_file(&netcfg_file).expect("no network config present");
        let storage = config.get_storage().unwrap();
        let addresses_bytes : Vec<_> = values_t!(args.values_of("addresses"), String)
            .unwrap().iter().map(|s| base58::decode(s).unwrap()).collect();
        let mut addresses : Vec<ExtendedAddr> = vec![];
        for address in addresses_bytes {
            addresses.push(cbor::decode_from_cbor(&address).unwrap());
        }
        let mut epoch_id = 0;
        while let Some(h) = tag::read_hash(&storage, &tag::get_epoch_tag(epoch_id)) {
            info!("looking in epoch {}", epoch_id);
            let mut reader = pack::PackReader::init(&storage.config, &h.into_bytes());
            while let Some(blk_bytes) = reader.get_next() {
                let blk : Block = cbor::decode_from_cbor(&blk_bytes).unwrap();
                let hdr = blk.get_header();
                let blk_hash = hdr.compute_hash();
                debug!("  looking at slot {}", hdr.get_slotid().slotid);
                match blk {
                    Block::GenesisBlock(_) => {
                        debug!("    ignoring genesis block")
                    },
                    Block::MainBlock(mblk) => {
                        for txaux in mblk.body.tx.iter() {
                            for txout in &txaux.tx.outputs {
                                if let Some(_) = addresses.iter().find(|a| *a == &txout.address) {
                                    println!("found address: {} in block {} at Epoch {} SlotId {}",
                                        base58::encode(&cbor::encode_to_cbor(&txout.address).unwrap()),
                                        blk_hash,
                                        hdr.get_slotid().epoch,
                                        hdr.get_slotid().slotid,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            epoch_id += 1;
        }
    }
}
