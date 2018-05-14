#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate serde_yaml;
extern crate rcw;
extern crate wallet_crypto;
extern crate exe_common;
extern crate blockchain;
extern crate protocol;
extern crate storage;
extern crate rand;
extern crate ansi_term;
extern crate termion;
extern crate flate2;

mod config;
mod account;
mod command;
mod wallet;
mod block;

use config::{Config};
use command::{HasCommand};
use wallet::{Wallet};
use exe_common::network::{Network};
use block::{Block};

use std::env::{home_dir};
use std::path::{PathBuf};

fn main() {
    use clap::{App, Arg};

    env_logger::init();
    trace!("Starting application, {}-{}", crate_name!(), crate_version!());

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(Arg::with_name("config").short("c").long("config").value_name("FILE").help("Sets a custom config file").takes_value(true))
        .subcommand(Config::mk_command())
        .subcommand(Wallet::mk_command())
        .subcommand(Network::mk_command())
        .subcommand(Block::mk_command())
        .get_matches();

    let cfg_path = matches.value_of("config")
        .map_or(get_default_config(), |s| PathBuf::from(s));
    let cfg = Config::from_file(&cfg_path);

    match matches.subcommand() {
        (Config::COMMAND, Some(sub_matches)) => {
            if let Some(cfg2) = Config::run(cfg, sub_matches) {
                cfg2.to_file(&cfg_path);
            };
        },
        (Wallet::COMMAND, Some(sub_matches)) => {
            if let Some(cfg2) = Wallet::run(cfg, sub_matches) {
                cfg2.to_file(&cfg_path);
            };
        },
        (Network::COMMAND, Some(sub_matches)) => { Network::run((), sub_matches); },
        (Block::COMMAND,   Some(sub_matches)) => { Block::run(cfg, sub_matches); },
        _ => {
            println!("{}", matches.usage());
            ::std::process::exit(1);
        },
    }
}

fn get_default_config() -> PathBuf {
    match home_dir() {
        None => panic!("Unable to retrieve your home directory, set the --config option"),
        Some(mut d) => {d.push(".ariadne/wallet.yml"); d }
    }
}
