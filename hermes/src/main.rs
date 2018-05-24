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
extern crate exe_common;

use std::{sync::{Arc}, path::{PathBuf}};

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
        .subcommand(
            SubCommand::with_name("init")
                .about("init hermes environment")
                .arg(Arg::with_name("PORT NUMBER")
                    .long("port")
                    .takes_value(true)
                    .value_name("PORT NUMBER")
                    .help("set the port number to listen to")
                    .required(false)
                    .default_value(r"80")
                )
                .arg(Arg::with_name("NETWORKS DIRECTORY")
                    .long("networks-dir")
                    .takes_value(true)
                    .value_name("NETWORKS DIRECTORY")
                    .help("the relative or absolute directory of the networks to server")
                    .required(false)
                    .default_value(r"networks")
                )
        )
        .subcommand(
            SubCommand::with_name("start")
                .about("start explorer server")
        )
        .get_matches();

    let mut cfg = Config::open().unwrap_or(Config::default());

    match matches.subcommand() {
        ("init", Some(args)) => {
            let port = value_t!(args.value_of("PORT NUMBER"), u16).unwrap();
            let dir  = value_t!(args.value_of("NETWORKS DIRECTORY"), String).unwrap();
            cfg.port = port;
            cfg.root_dir = PathBuf::from(&dir);
            cfg.save().unwrap();
        },
        ("start", _) => {
            info!("Starting {}-{}", crate_name!(), crate_version!());
            let mut router = router::Router::new();
            let networks = Arc::new(cfg.get_networks().unwrap());
            handlers::block::Handler::new(networks.clone()).route(&mut router);
            handlers::pack::Handler::new(networks.clone()).route(&mut router);
            handlers::epoch::Handler::new(networks.clone()).route(&mut router);
            info!("listenting to port {}", cfg.port);
            Iron::new(router).http(format!("localhost:{}", cfg.port)).unwrap();
        },
        _ => {
            println!("{}", matches.usage());
            ::std::process::exit(1);
        },
    }
}
