use exe_common::{config::{net}, network::{Peer, api::{*}}};
use clap::{ArgMatches, Arg};

pub mod range {
    use std::str::FromStr;

    #[derive(Debug)]
    pub struct RangeOption {
        pub from: String,
        pub to: Option<String>
    }

    #[derive(Debug)]
    pub enum Error {
        Empty,
        InvalidRange
    }

    impl FromStr for RangeOption {
        type Err = Error;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            if s.is_empty() { return Err(Error::Empty); }

            let mut v : Vec<&str> = s.split("..").collect();
            if v.is_empty() || v.len() > 2 { return Err(Error::InvalidRange); }

            let h1 = v.pop().unwrap().to_string(); // we expect at least one
            if let Some(h2) = v.pop().map(|v| v.to_string()) {
                Ok(RangeOption { from: h2, to: Some(h1)})
            } else {
                Ok(RangeOption { from: h1, to: None})
            }
        }
    }
}

use config::Config; // TODO, remove me

pub fn blockchain_name_arg<'a, 'b>(index: u64) -> Arg<'a,'b> {
    Arg::with_name("name")
        .help("the blockchain name")
        .index(index)
        .required(true)
}

pub fn resolv_network_by_name<'a>(opts: &ArgMatches<'a>) -> Config {
    let name = value_t!(opts.value_of("name"), String).unwrap();
    let mut config = Config::default();
    config.network = name;
    config
}

pub fn get_native_peer(blockchain: String, cfg: &net::Config) -> Peer {
    for peer in cfg.peers.iter() {
        if peer.is_native() {
            return Peer::new(blockchain, peer.name().to_owned(), peer.peer().clone(), cfg.protocol_magic).unwrap()
        }
    }

    panic!("no native peer to connect to")
}

pub fn get_http_peer(blockchain: String, cfg: &net::Config) -> Peer {
    for peer in cfg.peers.iter() {
        if peer.is_http() {
            return Peer::new(blockchain, peer.name().to_owned(), peer.peer().clone(), cfg.protocol_magic).unwrap()
        }
    }

    panic!("no http peer to connect to")
}
