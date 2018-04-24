#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
extern crate serde_yaml;
extern crate wallet_crypto;
extern crate rand;

mod config;
mod account;
mod command;
mod wallet;

use config::{Config};
use command::{HasCommand};
use wallet::{Wallet};

use std::env::{home_dir};
use std::path::{PathBuf};

fn main() {
    use clap::{App, Arg};

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::with_name("config").short("c").long("config").value_name("FILE").help("Sets a custom config file").takes_value(true))
        .subcommand(Config::clap_options())
        .subcommand(Wallet::clap_options())
        .get_matches();

    let cfg_path = matches.value_of("config")
        .map_or(get_default_config(), |s| PathBuf::from(s));
    let cfg = Config::from_file(&cfg_path);

    match matches.subcommand() {
        ("config", Some(sub_matches)) => {
            if let Some(cfg2) = Config::run(cfg, sub_matches) {
                cfg2.to_file(&cfg_path);
            };
        },
        ("wallet", Some(sub_matches)) => {
            if let Some(cfg2) = Wallet::run(cfg, sub_matches) {
                cfg2.to_file(&cfg_path);
            };
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
        Some(mut d) => {d.push(".ariadne-wallet.yml"); d }
    }
}
