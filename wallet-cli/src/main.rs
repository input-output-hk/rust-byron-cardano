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

use command::{HasCommand};

fn main() {
    use clap::{App};

    env_logger::init();
    trace!("Starting application, {}-{}", crate_name!(), crate_version!());

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommand(command::Wallet::mk_command())
        .subcommand(command::Blockchain::mk_command())
        .get_matches();

    match matches.subcommand() {
        (command::Wallet::COMMAND,     Some(sub_matches)) => command::Wallet::run((), sub_matches),
        (command::Blockchain::COMMAND, Some(sub_matches)) => command::Blockchain::run((), sub_matches),
        _ => {
            println!("{}", matches.usage());
            ::std::process::exit(1);
        },
    }
}
