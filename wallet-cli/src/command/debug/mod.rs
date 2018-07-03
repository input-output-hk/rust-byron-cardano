use cardano::{util::{hex, base58}, address::ExtendedAddr};
use command::{HasCommand};
use clap::{ArgMatches, Arg, SubCommand, App};
use cardano::block;
use config::{Config};
use std::io::{Write, stdout};
use cbor_event::de::RawCbor;

use exe_common::{config::{net}, network::{api::{*}}, sync};

use command::pretty::Pretty;

pub struct Debug;

impl HasCommand for Debug {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "debug";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("debug operations")
            .subcommand(SubCommand::with_name("address")
                .about("deconstruct an address to its component")
                .arg(Arg::with_name("address").index(1).help("address in base58").required(true)
                )
            )
    }

    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        match args.subcommand() {
            ("address", Some(opts)) => {
                let hh_base58 = value_t!(opts.value_of("address"), String).unwrap();
                debug_address(&hh_base58)
            },
            _ => {

                println!("{}", args.usage());
                ::std::process::exit(3);
            },
        }
    }
}

fn debug_address(address_base58: &str) {
    let addr_raw = base58::decode(address_base58).unwrap();
    let addr = ExtendedAddr::from_bytes(&addr_raw[..]).unwrap();

    println!("addr: {}", addr.addr);
    println!("type: {:?}", addr.addr_type);
    println!("attributes:");
    println!("  derivation_path: {:?}", addr.attributes.derivation_path);
    println!("  stake_distribution: {:?}", addr.attributes.stake_distribution);
}
