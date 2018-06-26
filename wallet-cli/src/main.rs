#[macro_use]
extern crate clap;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
#[macro_use]
extern crate raw_cbor;
extern crate env_logger;
extern crate serde_yaml;
extern crate rcw;
extern crate cardano;
extern crate exe_common;
extern crate protocol;
extern crate storage;
extern crate rand;
extern crate ansi_term;
extern crate termion;
extern crate flate2;

mod command;
mod config;

use command::{HasCommand};

fn main() {
    use clap::{App};

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();
    trace!("Starting application, {}-{}", crate_name!(), crate_version!());

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommand(command::Wallet::mk_command())
        .subcommand(command::Blockchain::mk_command())
        .subcommand(command::Debug::mk_command())
        .get_matches();

    match matches.subcommand() {
        (command::Wallet::COMMAND,     Some(sub_matches)) => command::Wallet::run((), sub_matches),
        (command::Blockchain::COMMAND, Some(sub_matches)) => command::Blockchain::run((), sub_matches),
        (command::Debug::COMMAND,      Some(sub_matches)) => command::Debug::run((), sub_matches),
        _ => {
            println!("{}", matches.usage());
            ::std::process::exit(1);
        },
    }
}
