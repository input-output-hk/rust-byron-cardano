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

use config::Config;

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
                )
                .arg(Arg::with_name("NETWORKS DIRECTORY")
                    .long("networks-dir")
                    .takes_value(true)
                    .value_name("NETWORKS DIRECTORY")
                    .help("the relative or absolute directory of the networks to server")
                    .required(false)
                )
        )
        .get_matches();

    let mut cfg = Config::open().unwrap_or(Config::default());

    match matches.subcommand() {
        ("start", Some(args)) => {
            cfg.port = value_t!(args.value_of("PORT NUMBER"), u16)
                .or_else(|err| match err {
                    clap::Error{ kind:clap::ErrorKind::ArgumentNotFound, .. } => Ok(cfg.port),
                    err => Err(err),
                })
                .unwrap();
            cfg.root_dir = value_t!(args.value_of("NETWORKS DIRECTORY"), String)
                .map(PathBuf::from)
                .or_else(|err| match err {
                    clap::Error{ kind:clap::ErrorKind::ArgumentNotFound, .. } => Ok(cfg.root_dir.clone()),
                    err => Err(err),
                })
                .unwrap();
            ::std::fs::create_dir_all(cfg.root_dir.clone()).expect("create networks directory");
            info!("Created networks directory {:?}", cfg.root_dir);
            info!("Starting {}-{}", crate_name!(), crate_version!());
            service::start(cfg);
        },
        _ => {
            println!("{}", matches.usage());
            ::std::process::exit(1);
        },
    }
}
