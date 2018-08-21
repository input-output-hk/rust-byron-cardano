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
extern crate cardano;
extern crate exe_common;

use std::path::{PathBuf};


mod config;
mod handlers;
mod service;

use config::{Config, hermes_path};
use exe_common::config::net;

fn main() {
    use clap::{App, Arg, SubCommand};

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommand(
            SubCommand::with_name("start")
                .about("start explorer server")
                .arg(Arg::with_name("PORT NUMBER")
                    .long("port")
                    .takes_value(true)
                    .value_name("PORT NUMBER")
                    .help("set the port number to listen to")
                    .required(false)
                    .default_value("80")
                )
                .arg(Arg::with_name("NETWORKS DIRECTORY")
                    .long("networks-dir")
                    .takes_value(true)
                    .value_name("NETWORKS DIRECTORY")
                    .help("the relative or absolute directory of the networks to server")
                    .required(false)
                )
                .arg(Arg::with_name("TEMPLATE")
                    .long("template")
                    .takes_value(true)
                    .value_name("TEMPLATE")
                    .help("either 'mainnet' or 'testnet'; may be given multiple times")
                    .required(false)
                    .multiple(true)
                    .default_value("mainnet")
                    .possible_values(&["mainnet", "staging", "testnet"])
                )
                .arg(Arg::with_name("no-sync")
                    .long("no-sync")
                    .help("disable synchronizing with the upstream network")
                )
        )
        .get_matches();

    match matches.subcommand() {
        ("start", Some(args)) => {

            let mut cfg = Config::new(
                PathBuf::from(
                    value_t!(args.value_of("NETWORKS DIRECTORY"), String)
                    .unwrap_or(
                        hermes_path().unwrap().join("networks")
                            .to_str().unwrap().to_string())),
                value_t!(args.value_of("PORT NUMBER"), u16).unwrap());

            ::std::fs::create_dir_all(cfg.root_dir.clone()).expect("create networks directory");
            info!("Created networks directory {:?}", cfg.root_dir);

            for template in args.values_of("TEMPLATE").unwrap() {
                let net_cfg = match template {
                    "mainnet" => { net::Config::mainnet() },
                    "staging" => { net::Config::staging() },
                    "testnet" => { net::Config::testnet() },
                    _         => {
                        // we do not support custom template yet.
                        // in the mean while the error is handled by clap
                        // (possible_values)
                        panic!("unknown template '{}'", template)
                    }
                };

                cfg.add_network(template, &net_cfg).unwrap();
            }

            cfg.sync = !args.is_present("no-sync");

            info!("Starting {}-{}", crate_name!(), crate_version!());
            service::start(cfg);
        },
        _ => {
            println!("{}", matches.usage());
            ::std::process::exit(1);
        },
    }
}
