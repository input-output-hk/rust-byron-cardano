use std::path::PathBuf;

extern crate dirs;
extern crate cardano_cli;

use self::cardano_cli::utils::term;
use self::cardano_cli::{blockchain, wallet, debug};

#[macro_use]
extern crate clap;
use clap::{Arg, App, SubCommand, ArgMatches};

fn main() {
    let default_root_dir = get_default_root_dir();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())

        .arg(global_quiet_definition())
        .arg(global_color_definition())
        .arg(global_rootdir_definition(&default_root_dir))

        .subcommand(blockchain_commands_definition())
        .subcommand(wallet_commands_definition())
        .subcommand(debug_commands_definition())
        .get_matches();

    let mut term = term::Term::new(configure_terminal(&matches));

    let root_dir = global_rootdir_match(&default_root_dir, &matches);

    match matches.subcommand() {
        ("blockchain", Some(matches)) => {
            subcommand_blockchain(term, root_dir, matches)
        },
        ("wallet", Some(matches)) => {
            subcommand_wallet(term, root_dir, matches)
        },
        ("debug", Some(matches)) => {
            subcommand_debug(term, root_dir, matches)
        },
        _ => {
            term.error(matches.usage()).unwrap();
            ::std::process::exit(1)
        }
    }
}

/* ------------------------------------------------------------------------- *
 *            Global options and helpers                                     *
 * ------------------------------------------------------------------------- */

const APPLICATION_DIRECTORY_NAME : &'static str = "cardano-cli";
const APPLICATION_ENVIRONMENT_ROOT_DIR : &'static str = "CARDANO_CLI_ROOT_DIR";

fn get_default_root_dir() -> PathBuf {
    match dirs::data_local_dir() {
        None      => { unimplemented!()   },
        Some(dir) => dir.join(APPLICATION_DIRECTORY_NAME)
    }
}
fn global_rootdir_definition<'a, 'b>(default: &'a PathBuf) -> Arg<'a, 'b> {
    Arg::with_name("ROOT_DIR")
        .long("root-dir")
        .help("the project root direction")
        .default_value(default.to_str().unwrap())
        .env(APPLICATION_ENVIRONMENT_ROOT_DIR)
}
fn global_rootdir_match<'a>(default: &'a PathBuf, matches: &ArgMatches<'a>) -> PathBuf {
    match matches.value_of("ROOT_DIR") {
        Some(dir) => { PathBuf::from(dir) },

        // technically the None option should not be needed
        // as we have already specified a default value
        // when defining the command line argument
        None => { PathBuf::from(default) },
    }
}

fn global_quiet_definition<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("QUIET")
        .long("quiet")
        .global(true)
        .help("run the command quietly, do not print anything to the command line output")
}
fn global_quiet_option<'a>(matches: &ArgMatches<'a>) -> bool {
    matches.is_present("QUIET")
}

fn global_color_definition<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("COLOR")
        .long("color")
        .takes_value(true)
        .default_value("auto")
        .possible_values(&["auto", "always", "never"])
        .global(true)
        .help("enable output colors or not")
}
fn global_color_option<'a>(matches: &ArgMatches<'a>) -> term::ColorChoice {
    match matches.value_of("COLOR") {
        None            => term::ColorChoice::Auto,
        Some("auto")    => term::ColorChoice::Auto,
        Some("always")  => term::ColorChoice::Always,
        Some("never")   => term::ColorChoice::Never,
        Some(&_) => {
            // this should not be reachable `clap` will perform validation
            // checking of the possible_values given when creating the argument
            unreachable!()
        }
    }
}

fn configure_terminal<'a>(matches: &ArgMatches<'a>) -> term::Config {
    term::Config {
        color: global_color_option(matches),
        quiet: global_quiet_option(matches)
    }
}

/* ------------------------------------------------------------------------- *
 *            Blockchain Sub Commands and helpers                            *
 * ------------------------------------------------------------------------- */

const BLOCKCHAIN_COMMAND : &'static str = "blockchain";

fn blockchain_argument_name_definition<'a, 'b>() -> Arg<'a,'b> {
    Arg::with_name("BLOCKCHAIN_NAME")
        .help("the blockchain name")
        .required(true)
}
fn blockchain_argument_name_match<'a>(matches: &ArgMatches<'a>) -> String {
    match matches.value_of("BLOCKCHAIN_NAME") {
        Some(r) => { r.to_owned() },
        None => { unreachable!() }
    }
}
fn blockchain_argument_remote_alias_definition<'a, 'b>() -> Arg<'a,'b> {
    Arg::with_name("BLOCKCHAIN_REMOTE_ALIAS")
        .help("Alias given to a remote node.")
        .required(true)
}
fn blockchain_argument_remote_alias_match<'a>(matches: &ArgMatches<'a>) -> String {
    match matches.value_of("BLOCKCHAIN_REMOTE_ALIAS") {
        Some(r) => { r.to_owned() },
        None => { unreachable!() }
    }
}
fn blockchain_argument_remote_endpoint_definition<'a, 'b>() -> Arg<'a,'b> {
    Arg::with_name("BLOCKCHAIN_REMOTE_ENDPOINT")
        .help("Remote end point (IPv4 or IPv6 address or domain name. May include a port number. And a sub-route point in case of an http endpoint.")
        .required(true)
}
fn blockchain_argument_remote_endpoint_match<'a>(matches: &ArgMatches<'a>) -> String {
    match matches.value_of("BLOCKCHAIN_REMOTE_ENDPOINT") {
        Some(r) => { r.to_owned() },
        None => { unreachable!() }
    }
}
fn blockchain_argument_template_definition<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("BLOCKCHAIN_TEMPLATE")
        .long("template")
        .value_name("TEMPLATE")
        .help("the template for the new blockchain")
        .required(false)
        .possible_values(&["mainnet", "testnet"])
        .default_value("mainnet")
}
fn blockchain_argument_template_match<'a>(matches: &ArgMatches<'a>)
    -> blockchain::Config
{
    match matches.value_of("BLOCKCHAIN_TEMPLATE") {
        None => blockchain::Config::mainnet(),
        Some("mainnet") => blockchain::Config::mainnet(),
        Some("testnet") => blockchain::Config::testnet(),
        Some(&_) => {
            // this should not be reachable as clap is handling
            // checking the value against all possible value
            unreachable!()
        }
    }
}

fn subcommand_blockchain<'a>(mut term: term::Term, root_dir: PathBuf, matches: &ArgMatches<'a>) {
    match matches.subcommand() {
        ("new", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let net_config = blockchain_argument_template_match(&matches);

            blockchain::command_new(term, root_dir, name, net_config);
        },
        ("remote-add", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let alias = blockchain_argument_remote_alias_match(&matches);
            let endpoint = blockchain_argument_remote_endpoint_match(&matches);

            blockchain::command_remote_add(term, root_dir, name, alias, endpoint);
        },
        ("remote-rm", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let alias = blockchain_argument_remote_alias_match(&matches);

            blockchain::command_remote_rm(term, root_dir, name, alias);
        },
        _ => {
            term.error(matches.usage()).unwrap();
            ::std::process::exit(1)
        }
    }
}
fn blockchain_commands_definition<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(BLOCKCHAIN_COMMAND)
        .about("blockchain operations")
        .subcommand(SubCommand::with_name("new")
            .about("create a new local blockchain")
            .arg(blockchain_argument_template_definition())
            .arg(blockchain_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("remote-add")
            .about("Attach a remote node to the local blockchain, this will allow to sync the local blockchain with this remote node.")
            .arg(blockchain_argument_name_definition())
            .arg(blockchain_argument_remote_alias_definition())
            .arg(blockchain_argument_remote_endpoint_definition())
        )
        .subcommand(SubCommand::with_name("remote-rm")
            .about("Remove the given remote node from the local blockchain, we will no longer fetch blocks from this remote node.")
            .arg(blockchain_argument_name_definition())
            .arg(blockchain_argument_remote_alias_definition())
        )
        .subcommand(SubCommand::with_name("remote-fetch")
            .about("Fetch blocks from the remote nodes (optionally specified by the aliases).")
            .arg(blockchain_argument_name_definition())
            .arg(blockchain_argument_remote_alias_definition()
                .multiple(true) // we want to accept multiple aliases here too
            )
        )
        .subcommand(SubCommand::with_name("forward")
            .about("Forward the local tip to what seems to be the consensus within the remote blocks. This function must be used combined with `remote-fetch'.")
            .arg(blockchain_argument_name_definition())
            .arg(Arg::with_name("FORWARD_TO_BLOCK")
                .value_name("HASH")
                .required(false)
                .help("Set the new local tip to the given blockhash, do not try to figure out consensus between the remote nodes.")
            )
        )
        .subcommand(SubCommand::with_name("pull")
            .about("handy command to `remote-fetch' and `forward' the local blockchain.")
            .arg(blockchain_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("gc")
            .about("run garbage collection of lose blocks. This function might be a bit slow to run but it will free some disk space.")
            .arg(blockchain_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("cat")
            .about("print the content of a block.")
            .arg(blockchain_argument_name_definition())
            .arg(Arg::with_name("HASH_BLOCK")
                .value_name("HASH")
                .required(true)
                .help("The block hash to open.")
            )
        )
        .subcommand(SubCommand::with_name("status")
            .about("print some details about the given blockchain")
            .arg(blockchain_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("log")
            .about("print some details about the given blockchain")
            .arg(blockchain_argument_name_definition())
            .arg(Arg::with_name("HASH_BLOCK")
                .value_name("HASH")
                .required(false)
                .help("The hash to start from (instead of the local blockchain's tip).")
            )
        )
}

/* ------------------------------------------------------------------------- *
 *                Wallet Sub Commands and helpers                            *
 * ------------------------------------------------------------------------- */

fn wallet_argument_name_definition<'a, 'b>() -> Arg<'a,'b> {
    Arg::with_name("WALLET_NAME")
        .help("the wallet name")
        .required(true)
}
fn wallet_argument_name_match<'a>(matches: &ArgMatches<'a>) -> String {
    match matches.value_of("WALLET_NAME") {
        Some(r) => { r.to_owned() },
        None => { unreachable!() }
    }
}

const WALLET_COMMAND : &'static str = "wallet";

fn subcommand_wallet<'a>(mut term: term::Term, root_dir: PathBuf, matches: &ArgMatches<'a>) {
    match matches.subcommand() {
        ("new", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);

            wallet::command_new(term, root_dir, name);
        },
        _ => {
            term.error(matches.usage()).unwrap();
            ::std::process::exit(1)
        }
    }
}
fn wallet_commands_definition<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(WALLET_COMMAND)
        .about("wallet operations")
        .subcommand(SubCommand::with_name("create")
            .about("create a new wallet")
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("recover")
            .about("recover a wallet")
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("destroy")
            .about("delete all data associated to the given wallet.")
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("attach")
            .about("Attach the existing wallet to the existing local blockchain. Detach first to attach to an other blockchain.")
            .arg(wallet_argument_name_definition())
            .arg(blockchain_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("detach")
            .about("detach the wallet from its associated blockchain")
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("sync")
            .about("synchronize the wallet with the attached blockchain")
            .arg(Arg::with_name("DRY_RUN")
                .help("perform the sync without storing the updated states.")
                .long("dry-run")
            )
            .arg(Arg::with_name("SYNC_TO_HASH")
                .help("sync the wallet up to the given hash (otherwise, sync up to local blockchain's tip).")
                .long("to")
                .value_name("HASH")
                .takes_value(true)
            )
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("status")
            .about("print some status information from the given wallet (funds, transactions...)")
            .arg(wallet_argument_name_definition())
        )
}

/* ------------------------------------------------------------------------- *
 *                Debug Sub Commands and helpers                            *
 * ------------------------------------------------------------------------- */

const DEBUG_COMMAND : &'static str = "debug";

fn subcommand_debug<'a>(mut term: term::Term, _rootdir: PathBuf, matches: &ArgMatches<'a>) {
    match matches.subcommand() {
        ("address", Some(matches)) => {
            let address = value_t!(matches, "ADDRESS", String).unwrap_or_else(|e| e.exit() );

            debug::command_address(term, address);
        },
        _ => {
            term.error(matches.usage()).unwrap();
            ::std::process::exit(1)
        }
    }
}
fn debug_commands_definition<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(DEBUG_COMMAND)
        .about("Debug and advanced tooling operations.")
        .subcommand(SubCommand::with_name("address")
            .about("check if the given address (in base58) is valid and print information about it.")
            .arg(Arg::with_name("ADDRESS")
                .help("base58 encoded address")
                .value_name("ADDRESS")
                .required(true)
            )
        )
        .subcommand(SubCommand::with_name("log-dump")
            .about("pretty print the content of the wallet log file")
            .arg(Arg::with_name("LOG_FILE")
                .help("the path to the file to print logs from")
                .value_name("FILE")
                .required(true)
            )
        )
}
