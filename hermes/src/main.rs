#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
#[macro_use]
extern crate log;
extern crate env_logger;

extern crate iron;
extern crate router;

extern crate storage;
extern crate wallet_crypto;
extern crate blockchain;

use std::env::{home_dir};
use std::path::{PathBuf};
use std::sync::{Arc};

use iron::Iron;

mod config;
mod handlers;

use config::{Config};

fn main() {
    use clap::{App, Arg, SubCommand};

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::with_name("config").short("c").long("config").value_name("FILE").help("Sets a custom config file").takes_value(true))
        .subcommand(
            SubCommand::with_name("start")
                .about("start explorer server")
        )
        .get_matches();

    let cfg_path = matches.value_of("config")
        .map_or(get_default_config(), |s| PathBuf::from(s));
    let cfg = Config::from_file(&cfg_path);

    match matches.subcommand() {
        ("start", _) => {
            info!("Starting {}-{}", crate_name!(), crate_version!());
            info!("listenting to port 3000");
            let mut router = router::Router::new();
            let storage = Arc::new(cfg.get_storage().unwrap());
            handlers::block::Handler::new(storage.clone()).route(&mut router);
            handlers::pack::Handler::new(storage.clone()).route(&mut router);
            handlers::epoch::Handler::new(storage.clone()).route(&mut router);
            Iron::new(router).http("localhost:3000").unwrap();
        },
        _ => {
            println!("{}", matches.usage());
            ::std::process::exit(1);
        },
    }
}

fn get_default_config() -> PathBuf {
    match home_dir() {
        None => panic!("Unable to retrieve your home directory, set the --config option"),
        Some(mut d) => {d.push(".ariadne/explorer.yml"); d }
    }
}
