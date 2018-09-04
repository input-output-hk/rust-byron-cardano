use std::path::PathBuf;

extern crate dirs;
extern crate cardano_cli;
extern crate cardano;
extern crate log;
extern crate env_logger;

use self::cardano_cli::utils::term;
use self::cardano_cli::{blockchain, wallet, transaction, debug};

#[macro_use]
extern crate clap;
use clap::{Arg, App, SubCommand, ArgMatches};

fn main() {
    let default_root_dir = get_default_root_dir();

    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())

        .arg(global_verbose_definition())
        .arg(global_quiet_definition())
        .arg(global_color_definition())
        .arg(global_rootdir_definition(&default_root_dir))

        .subcommand(blockchain_commands_definition())
        .subcommand(wallet_commands_definition())
        .subcommand(transaction_commands_definition())
        .subcommand(debug_commands_definition())
        .get_matches();

    let mut term = term::Term::new(configure_terminal(&matches));

    let root_dir = global_rootdir_match(&default_root_dir, &matches);

    match matches.subcommand() {
        (BLOCKCHAIN_COMMAND, Some(matches))  => { subcommand_blockchain(term, root_dir, matches) },
        (WALLET_COMMAND, Some(matches))      => { subcommand_wallet(term, root_dir, matches) },
        (TRANSACTION_COMMAND, Some(matches)) => { subcommand_transaction(term, root_dir, matches) },
        (DEBUG_COMMAND, Some(matches))       => { subcommand_debug(term, root_dir, matches) },
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
fn global_verbose_definition<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("VERBOSITY")
        .long("verbose")
        .short("v")
        .multiple(true)
        .global(true)
        .help("set the verbosity mode, multiple occurrences means more verbosity")
}
fn global_verbose_option<'a>(matches: &ArgMatches<'a>) -> u64 {
    matches.occurrences_of("VERBOSITY")
}

fn configure_terminal<'a>(matches: &ArgMatches<'a>) -> term::Config {
    let quiet = global_quiet_option(matches);
    let color = global_color_option(matches);
    let verbosity = global_verbose_option(matches);

    if ! quiet {
        let log_level = match verbosity {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };
        env_logger::Builder::from_default_env()
            .filter_level(log_level)
            .init();
    }

    term::Config {
        color: color,
        quiet: quiet
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
        .possible_values(&["mainnet", "staging", "testnet"])
        .default_value("mainnet")
}
fn blockchain_argument_template_match<'a>(matches: &ArgMatches<'a>)
    -> blockchain::Config
{
    match matches.value_of("BLOCKCHAIN_TEMPLATE") {
        None => blockchain::Config::mainnet(),
        Some("mainnet") => blockchain::Config::mainnet(),
        Some("staging") => blockchain::Config::staging(),
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
        ("list", Some(matches)) => {
            let detailed = matches.is_present("LIST_DETAILS");

            blockchain::commands::list(term, root_dir, detailed);
        },
        ("new", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let net_config = blockchain_argument_template_match(&matches);

            blockchain::commands::new(term, root_dir, name, net_config);
        },
        ("remote-add", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let alias = blockchain_argument_remote_alias_match(&matches);
            let endpoint = blockchain_argument_remote_endpoint_match(&matches);

            blockchain::commands::remote_add(term, root_dir, name, alias, endpoint);
        },
        ("remote-rm", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let alias = blockchain_argument_remote_alias_match(&matches);

            blockchain::commands::remote_rm(term, root_dir, name, alias);
        },
        ("remote-fetch", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let peers = values_t!(matches, "BLOCKCHAIN_REMOTE_ALIAS", String).unwrap_or_else(|_| Vec::new());

            blockchain::commands::remote_fetch(term, root_dir, name, peers);
        },
        ("remote-ls", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let detailed = if matches.is_present("REMOTE_LS_DETAILED_SHORT") {
                blockchain::commands::RemoteDetail::Short
            } else if matches.is_present("REMOTE_LS_DETAILED_LOCAL") {
                blockchain::commands::RemoteDetail::Local
            } else if matches.is_present("REMOTE_LS_DETAILED_REMOTE") {
                blockchain::commands::RemoteDetail::Remote
            } else {
                blockchain::commands::RemoteDetail::Short
            };

            blockchain::commands::remote_ls(term, root_dir, name, detailed);
        },
        ("forward", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let opt_hash = matches.value_of("FORWARD_TO_BLOCK").map(|s| s.to_owned());

            blockchain::commands::forward(term, root_dir, name, opt_hash);
        },
        ("pull", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);

            blockchain::commands::pull(term, root_dir, name);
        },
        ("cat", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let hash = matches.value_of("HASH_BLOCK").unwrap();
            let no_parse = matches.is_present("BLOCK_NO_PARSE");
            let debug = matches.is_present("DEBUG");

            blockchain::commands::cat(term, root_dir, name, hash, no_parse, debug);
        },
        ("status", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);

            blockchain::commands::status(term, root_dir, name);
        },
        ("destroy", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);

            blockchain::commands::destroy(term, root_dir, name);
        },
        ("log", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let hash = matches.value_of("HASH_BLOCK").map(|s| s.to_owned());

            blockchain::commands::log(term, root_dir, name, hash);
        },
        ("verify-block", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            let hash = matches.value_of("HASH_BLOCK").unwrap();

            blockchain::commands::verify_block(term, root_dir, name, hash);
        },
        ("verify", Some(matches)) => {
            let name = blockchain_argument_name_match(&matches);
            blockchain::commands::verify_chain(term, root_dir, name);
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
        .subcommand(SubCommand::with_name("list")
            .about("list local blockchains")
            .arg(Arg::with_name("LIST_DETAILS")
                .long("detailed")
                .short("l")
                .required(false)
                .takes_value(false)
                .help("display some information regarding the remotes")
            )
        )
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
                .required(false) // we allow user not to set any values here
            )
        )
        .subcommand(SubCommand::with_name("remote-ls")
            .about("List all the remote nodes of the given blockchain")
            .arg(blockchain_argument_name_definition())
            .arg(Arg::with_name("REMOTE_LS_DETAILED_SHORT")
                .long("--short")
                .group("REMOTE_LS_DETAILED")
                .required(false)
                .help("print only the bare minimum information regarding the remotes (default)")
            )
            .arg(Arg::with_name("REMOTE_LS_DETAILED_LOCAL")
                .long("--detailed")
                .group("REMOTE_LS_DETAILED")
                .required(false)
                .help("print all local known information regarding the remotes")
            )
            .arg(Arg::with_name("REMOTE_LS_DETAILED_REMOTE")
                .long("--complete")
                .group("REMOTE_LS_DETAILED")
                .required(false)
                .help("print all local known information regarding the remotes as well as the details from the remote (needs a network connection)")
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
            .arg(Arg::with_name("BLOCK_NO_PARSE")
                .long("no-parse")
                .help("don't parse the block, flush the bytes direct to the standard output (not subject to `--quiet' option)")
            )
            .arg(Arg::with_name("DEBUG")
                .long("debug")
                .help("dump the block in debug format")
            )
        )
        .subcommand(SubCommand::with_name("status")
            .about("print some details about the given blockchain")
            .arg(blockchain_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("destroy")
            .about("destroy the given blockchain, deleting all the blocks downloaded from the disk.")
            .arg(blockchain_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("log")
            .about("print the block, one by one, from the given blockhash or the tip of the blockchain.")
            .arg(blockchain_argument_name_definition())
            .arg(Arg::with_name("HASH_BLOCK")
                .value_name("HASH")
                .required(false)
                .help("The hash to start from (instead of the local blockchain's tip).")
            )
        )
        .subcommand(SubCommand::with_name("verify-block")
            .about("verify the specified block")
            .arg(blockchain_argument_name_definition())
            .arg(Arg::with_name("HASH_BLOCK")
                .value_name("HASH")
                .required(true)
                .help("The hash of the block to verify.")
            )
        )
        .subcommand(SubCommand::with_name("verify")
            .about("verify all blocks in the chain")
            .arg(blockchain_argument_name_definition())
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
fn wallet_argument_name_match<'a>(matches: &ArgMatches<'a>) -> wallet::WalletName {
    match matches.value_of("WALLET_NAME") {
        Some(r) => { wallet::WalletName::new(r.to_owned()).expect("Wallet name is invalid. cannot contains . and /") },
        None => { unreachable!() }
    }
}
fn wallet_argument_wallet_scheme<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("WALLET_SCHEME")
        .help("the scheme to organize accounts and addresses in a Wallet.")
        .long("wallet-scheme")
        .takes_value(true)
        .possible_values(&["bip44", "random_index_2levels"])
        .default_value("bip44")
}
fn wallet_argument_wallet_scheme_match<'a>(matches: &ArgMatches<'a>) -> wallet::HDWalletModel {
    match matches.value_of("WALLET_SCHEME") {
        Some("bip44")                => wallet::HDWalletModel::BIP44,
        Some("random_index_2levels") => wallet::HDWalletModel::RandomIndex2Levels,
        _ => unreachable!() // default is "bip44"
    }
}
fn wallet_argument_mnemonic_languages<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("MNEMONIC_LANGUAGES")
        .help("the list of languages to display the mnemonic words of the wallet in. You can set multiple values using comma delimiter (example: `--mnemonics-languages=english,french,italian').")
        .long("mnemonics-languages")
        .takes_value(true)
        .use_delimiter(true)
        .require_delimiter(true)
        .value_delimiter(",")
        .possible_values(&["chinese-simplified", "chinese-traditional", "english", "french", "italian", "japanese", "korean", "spanish"])
        .default_value("english")
}
fn wallet_argument_mnemonic_languages_match<'a>(matches: &ArgMatches<'a>)
    -> Vec<impl cardano::bip::bip39::dictionary::Language>
{
    let mut languages = Vec::new();
    for lan in matches.values_of("MNEMONIC_LANGUAGES").unwrap() {
        let value = match lan {
            "chinese-simplified"  => cardano::bip::bip39::dictionary::CHINESE_SIMPLIFIED,
            "chinese-traditional" => cardano::bip::bip39::dictionary::CHINESE_TRADITIONAL,
            "english"             => cardano::bip::bip39::dictionary::ENGLISH,
            "french"              => cardano::bip::bip39::dictionary::FRENCH,
            "italian"             => cardano::bip::bip39::dictionary::ITALIAN,
            "japanese"            => cardano::bip::bip39::dictionary::JAPANESE,
            "korean"              => cardano::bip::bip39::dictionary::KOREAN,
            "spanish"             => cardano::bip::bip39::dictionary::SPANISH,
            _ => unreachable!() // clap knows the default values
        };
        languages.push(value);
    }
    languages
}
fn wallet_argument_mnemonic_language<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("MNEMONIC_LANGUAGE")
        .help("the language of the mnemonic words to recover the wallet from.")
        .long("mnemonics-language")
        .takes_value(true)
        .possible_values(&["chinese-simplified", "chinese-traditional", "english", "french", "italian", "japanese", "korean", "spanish"])
        .default_value("english")
}
fn wallet_argument_mnemonic_language_match<'a>(matches: &ArgMatches<'a>)
    -> impl cardano::bip::bip39::dictionary::Language
{
    match matches.value_of("MNEMONIC_LANGUAGE").unwrap() {
        "chinese-simplified"  => cardano::bip::bip39::dictionary::CHINESE_SIMPLIFIED,
        "chinese-traditional" => cardano::bip::bip39::dictionary::CHINESE_TRADITIONAL,
        "english"             => cardano::bip::bip39::dictionary::ENGLISH,
        "french"              => cardano::bip::bip39::dictionary::FRENCH,
        "italian"             => cardano::bip::bip39::dictionary::ITALIAN,
        "japanese"            => cardano::bip::bip39::dictionary::JAPANESE,
        "korean"              => cardano::bip::bip39::dictionary::KOREAN,
        "spanish"             => cardano::bip::bip39::dictionary::SPANISH,
        _ => unreachable!() // clap knows the default values
    }
}
fn wallet_argument_derivation_scheme<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("DERIVATION_SCHEME")
        .help("derivation scheme")
        .long("derivation-scheme")
        .takes_value(true)
        .possible_values(&["v1", "v2"])
        .default_value("v2")
}
fn wallet_argument_derivation_scheme_match<'a>(matches: &ArgMatches<'a>) -> cardano::hdwallet::DerivationScheme {
    match matches.value_of("DERIVATION_SCHEME") {
        Some("v1") => cardano::hdwallet::DerivationScheme::V1,
        Some("v2") => cardano::hdwallet::DerivationScheme::V2,
        _ => unreachable!() // default is "v2"
    }
}
fn wallet_argument_mnemonic_size<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("MNEMONIC_SIZE")
        .help("The number of words to use for the wallet mnemonic (the more the more secure).")
        .long("mnemonics-length")
        .takes_value(true)
        .possible_values(&["12", "15", "18", "21", "24"])
        .default_value("24")
}
fn wallet_argument_mnemonic_size_match<'a>(matches: &ArgMatches<'a>) -> cardano::bip::bip39::Type {
    match matches.value_of("MNEMONIC_SIZE") {
        Some("12") => cardano::bip::bip39::Type::Type12Words,
        Some("15") => cardano::bip::bip39::Type::Type15Words,
        Some("18") => cardano::bip::bip39::Type::Type18Words,
        Some("21") => cardano::bip::bip39::Type::Type21Words,
        Some("24") => cardano::bip::bip39::Type::Type24Words,
        _ => unreachable!() // default is "24"
    }
}
fn wallet_argument_daedalus_seed<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("DAEDALUS_SEED")
        .help("To recover a wallet generated from daedalus")
        .long("daedalus-seed")
        .takes_value(false)
}
fn wallet_argument_daedalus_seed_match<'a>(matches: &ArgMatches<'a>) -> bool {
    matches.is_present("DAEDALUS_SEED")
}

const WALLET_COMMAND : &'static str = "wallet";

fn subcommand_wallet<'a>(mut term: term::Term, root_dir: PathBuf, matches: &ArgMatches<'a>) {
    match matches.subcommand() {
        ("create", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);
            let wallet_scheme = wallet_argument_wallet_scheme_match(&matches);
            let derivation_scheme = wallet_argument_derivation_scheme_match(&matches);
            let mnemonic_length = wallet_argument_mnemonic_size_match(&matches);
            let mnemonic_langs  = wallet_argument_mnemonic_languages_match(&matches);

            wallet::commands::new(term, root_dir, name, wallet_scheme, derivation_scheme, mnemonic_length, mnemonic_langs);
        },
        ("recover", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);
            let mut wallet_scheme = wallet_argument_wallet_scheme_match(&matches);
            let mut derivation_scheme = wallet_argument_derivation_scheme_match(&matches);
            let mut mnemonic_length = wallet_argument_mnemonic_size_match(&matches);
            let mnemonic_lang   = wallet_argument_mnemonic_language_match(&matches);
            let daedalus_seed   = wallet_argument_daedalus_seed_match(&matches);
            let interactive = matches.is_present("RECOVER_INTERACTIVE");

            if daedalus_seed {
                if wallet_scheme != wallet::HDWalletModel::RandomIndex2Levels {
                    term.warn("Daedalus wallet are usually using `--wallet-scheme=random_index_2levels'\n").unwrap();
                }
                if derivation_scheme != cardano::hdwallet::DerivationScheme::V1 {
                    term.warn("Daedalus wallet are usually using `--derivation-scheme=v1'\n").unwrap();
                }
                if mnemonic_length != cardano::bip::bip39::Type::Type12Words {
                    term.warn("Daedalus wallet are usually using `--mnemonics-length=12'\n").unwrap();
                }
            }

            wallet::commands::recover(term, root_dir, name, wallet_scheme, derivation_scheme, mnemonic_length, interactive, daedalus_seed, mnemonic_lang);
        },
        ("address", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);
            let account = value_t!(matches, "ACCOUNT_INDEX", u32).unwrap_or_else(|e| e.exit());
            let index   = value_t!(matches, "ADDRESS_INDEX", u32).unwrap_or_else(|e| e.exit());
            let is_internal = matches.is_present("INTERNAL_ADDRESS");

            wallet::commands::address(term, root_dir, name, account, is_internal, index);
        },
        ("attach", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);
            let blockchain = blockchain_argument_name_match(&matches);

            wallet::commands::attach(term, root_dir, name, blockchain);
        },
        ("detach", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);

            wallet::commands::detach(term, root_dir, name);
        },
        ("sync", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);

            wallet::commands::sync(term, root_dir, name);
        },
        ("status", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);

            wallet::commands::status(term, root_dir, name);
        },
        ("log", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);

            wallet::commands::log(term, root_dir, name, false);
        },
        ("utxos", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);

            wallet::commands::utxos(term, root_dir, name);
        },
        ("statement", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);

            wallet::commands::log(term, root_dir, name, true);
        },
        ("destroy", Some(matches)) => {
            let name = wallet_argument_name_match(&matches);

            wallet::commands::destroy(term, root_dir, name);
        },
        ("list", Some(matches)) => {
            let detailed = matches.is_present("WALLET_LIST_DETAILED");

            wallet::commands::list(term, root_dir, detailed);
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
        .subcommand(SubCommand::with_name("list")
            .about("list all the wallets available")
            .arg(Arg::with_name("WALLET_LIST_DETAILED")
                .long("detailed")
                .short("l")
                .help("display some metadata information of the wallet")
            )
        )
        .subcommand(SubCommand::with_name("create")
            .about("create a new wallet")
            .arg(wallet_argument_mnemonic_size())
            .arg(wallet_argument_derivation_scheme())
            .arg(wallet_argument_wallet_scheme())
            .arg(wallet_argument_mnemonic_languages())
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("recover")
            .about("recover a wallet")
            .arg(wallet_argument_name_definition())
            .arg(wallet_argument_mnemonic_size())
            .arg(wallet_argument_derivation_scheme())
            .arg(wallet_argument_wallet_scheme())
            .arg(wallet_argument_mnemonic_language())
            .arg(wallet_argument_daedalus_seed())
            .arg(Arg::with_name("RECOVER_INTERACTIVE")
                .help("use interactive mode for recovering the mnemonic words")
                .long("interactive")
                .short("i")
            )
        )
        .subcommand(SubCommand::with_name("destroy")
            .about("delete all data associated to the given wallet.")
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("address")
            .about("create a new address")
            .arg(wallet_argument_name_definition())
            .arg(Arg::with_name("ACCOUNT_INDEX").required(true))
            .arg(Arg::with_name("ADDRESS_INDEX").required(true))
            .arg(Arg::with_name("INTERNAL_ADDRESS").long("internal"))
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
        .subcommand(SubCommand::with_name("statement")
            .about("print the wallet statement")
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("log")
            .about("print the wallet logs")
            .arg(wallet_argument_name_definition())
        )
        .subcommand(SubCommand::with_name("utxos")
            .about("print the wallet's available funds")
            .arg(wallet_argument_name_definition())
        )
}

/* ------------------------------------------------------------------------- *
 *             Transaction Sub Commands and helpers                          *
 * ------------------------------------------------------------------------- */

const TRANSACTION_COMMAND : &'static str = "transaction";

#[derive(Debug,Clone,Copy)]
pub enum TransactionCmd {
    New, List, Destroy, Export, Import, Sign, InputSelect, AddChange, AddInput, AddOutput, RmInput, RmOutput, RmChange, Status,
}
impl TransactionCmd {
    pub fn as_string(self) -> &'static str {
        match self {
            TransactionCmd::New => "new",
            TransactionCmd::List => "list",
            TransactionCmd::Destroy => "destroy",
            TransactionCmd::Export => "export",
            TransactionCmd::Import => "import",
            TransactionCmd::Sign => "sign",
            TransactionCmd::InputSelect => "input-select",
            TransactionCmd::AddChange => "add-change",
            TransactionCmd::AddInput => "add-input",
            TransactionCmd::AddOutput => "add-output",
            TransactionCmd::RmInput => "rm-input",
            TransactionCmd::RmOutput => "rm-output",
            TransactionCmd::RmChange => "rm-change",
            TransactionCmd::Status => "status",
        }
    }
}

fn transaction_argument_name_definition<'a, 'b>() -> Arg<'a,'b> {
    Arg::with_name("TRANSACTION_ID")
        .help("the transaction staging identifier")
        .required(true)
}
fn transaction_argument_name_match<'a, 'b>(matches: &'b ArgMatches<'a>) -> &'b str {
    match matches.value_of("TRANSACTION_ID") {
        Some(r) => { r },
        None => { unreachable!() }
    }
}
fn transaction_argument_txid_definition<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("TRANSACTION_TXID")
        .help("A Transaction identifier in hexadecimal")
        .required(false)
        .requires("TRANSACTION_INDEX")
}
fn transaction_argument_index_definition<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("TRANSACTION_INDEX")
        .help("The index of the unspent output in the transaction")
}
fn transaction_argument_amount_definition<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name("TRANSACTION_AMOUNT")
        .help("The value in lovelace")
}
fn transaction_argument_txin_match<'a>(matches: &ArgMatches<'a>) -> Option<(cardano::tx::TxId, u32)> {
    if ! matches.is_present("TRANSACTION_TXID") { return None; }
    let txid = value_t!(matches, "TRANSACTION_TXID", cardano::tx::TxId).unwrap_or_else(|e| e.exit());

    let index = value_t!(matches, "TRANSACTION_INDEX", u32).unwrap_or_else(|e| e.exit());

    Some((txid, index))
}
fn transaction_argument_input_match<'a>(matches: &ArgMatches<'a>) -> Option<(cardano::tx::TxId, u32, Option<cardano::coin::Coin>)> {
    let (txid, index) = transaction_argument_txin_match(&matches)?;
    let coin = value_t!(matches, "UTXO_AMOUNT", cardano::coin::Coin).ok();

    Some((txid, index, coin))
}
fn transaction_argument_address_definition<'a, 'b>() -> Arg<'a, 'b>
{
    Arg::with_name("TRANSACTION_ADDRESS")
        .help("Address to send funds too")
}
fn transaction_argument_output_match<'a>(matches: &ArgMatches<'a>) -> Option<(cardano::address::ExtendedAddr, cardano::coin::Coin)> {
    if ! matches.is_present("TRANSACTION_ADDRESS") { return None; }

    let address = value_t!(matches, "TRANSACTION_ADDRESS", cardano::address::ExtendedAddr).unwrap_or_else(|e| e.exit());
    let coin = value_t!(matches, "TRANSACTION_AMOUNT", cardano::coin::Coin).unwrap_or_else(|e| e.exit());

    Some((address, coin))
}

fn subcommand_transaction<'a>(mut term: term::Term, root_dir: PathBuf, matches: &ArgMatches<'a>) {
    match matches.subcommand() {
        ("new", Some(matches)) => {
            let blockchain = blockchain_argument_name_match(&matches);
            transaction::commands::new(term, root_dir, blockchain);
        },
        ("list", _) => {
            transaction::commands::list(term, root_dir);
        },
        ("destroy", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            transaction::commands::destroy(term, root_dir, id);
        },
        ("export", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            let file = matches.value_of("EXPORT_FILE");
            transaction::commands::export(term, root_dir, id, file);
        },
        ("import", Some(matches)) => {
            let file = matches.value_of("IMPORT_FILE");
            transaction::commands::import(term, root_dir, file);
        },
        ("sign", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);

            transaction::commands::sign(term, root_dir, id);
        },
        ("add-input", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            let input = transaction_argument_input_match(&matches);

            transaction::commands::add_input(term, root_dir, id, input);
        },
        ("add-output", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            let output = transaction_argument_output_match(&matches);

            transaction::commands::add_output(term, root_dir, id, output);
        },
        ("add-change", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            let address = value_t!(matches, "CHANGE_ADDRESS", cardano::address::ExtendedAddr).unwrap_or_else(|e| e.exit());

            transaction::commands::add_change(term, root_dir, id, address);
        },
        ("input-select", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            let wallets = values_t!(matches, "WALLET_NAME", wallet::WalletName).unwrap_or_else(|e| e.exit());

            transaction::commands::input_select(term, root_dir, id, wallets);
        }
        ("rm-output", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            let address = value_t!(matches, "TRANSACTION_ADDRESS", cardano::address::ExtendedAddr).ok();

            transaction::commands::remove_output(term, root_dir, id, address);
        },
        ("rm-input", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            let txin = transaction_argument_txin_match(&matches);

            transaction::commands::remove_input(term, root_dir, id, txin);
        },
        ("rm-change", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            let address = value_t!(matches, "CHANGE_ADDRESS", cardano::address::ExtendedAddr).unwrap_or_else(|e| e.exit());

            transaction::commands::remove_change(term, root_dir, id, address);
        },
        ("status", Some(matches)) => {
            let id = transaction_argument_name_match(&matches);
            transaction::commands::status(term, root_dir, id);
        },
        _ => {
            term.error(matches.usage()).unwrap();
            ::std::process::exit(1)
        }
    }
}
fn transaction_commands_definition<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(TRANSACTION_COMMAND)
        .about("Transaction operations.")
        .subcommand(SubCommand::with_name(TransactionCmd::New.as_string())
            .about("Create a new empty staging transaction")
            .arg(blockchain_argument_name_definition()
                .help("Transaction are linked to a blockchain to be valid")
            )
        )
        .subcommand(SubCommand::with_name(TransactionCmd::List.as_string())
            .about("List all staging transactions open")
        )
        .subcommand(SubCommand::with_name(TransactionCmd::Destroy.as_string())
            .about("Destroy a staging transaction")
            .arg(transaction_argument_name_definition())
        )
        .subcommand(SubCommand::with_name(TransactionCmd::Export.as_string())
            .about("Export a staging transaction for transfer into a human readable format")
            .arg(transaction_argument_name_definition())
            .arg(Arg::with_name("EXPORT_FILE")
                .help("optional file to export the staging transaction to (default will display the export to stdout)")
                .required(false)
            )
        )
        .subcommand(SubCommand::with_name(TransactionCmd::Import.as_string())
            .about("Import a human readable format transaction into a new staging transaction")
            .arg(Arg::with_name("IMPORT_FILE")
                .help("optional file to import the staging transaction from (default will read stdin)")
                .required(false)
            )
        )
        .subcommand(SubCommand::with_name(TransactionCmd::Sign.as_string())
            .about("Finalize a staging a transaction into a transaction ready to send to the blockchain network")
            .arg(transaction_argument_name_definition())
        )
        .subcommand(SubCommand::with_name(TransactionCmd::InputSelect.as_string())
            .about("Select input automatically using a wallet (or a set of wallets), and a input selection algorithm")
            .arg(transaction_argument_name_definition())
            .arg(Arg::with_name("WALLET_NAME").required(true).multiple(true).help("wallet name to use for the selection"))
        )
        .subcommand(SubCommand::with_name(TransactionCmd::AddChange.as_string())
            .about("Add a change address to a transaction")
            .arg(transaction_argument_name_definition())
            .arg(Arg::with_name("CHANGE_ADDRESS").required(true).help("address to send the change to"))
        )
        .subcommand(SubCommand::with_name(TransactionCmd::RmChange.as_string())
            .about("Remove a change address from a transaction")
            .arg(transaction_argument_name_definition())
            .arg(Arg::with_name("CHANGE_ADDRESS").required(true).help("address to remove"))
        )
        .subcommand(SubCommand::with_name(TransactionCmd::AddInput.as_string())
            .about("Add an input to a transaction")
            .arg(transaction_argument_name_definition())
            .arg(transaction_argument_txid_definition())
            .arg(transaction_argument_index_definition())
            .arg(transaction_argument_amount_definition())
        )
        .subcommand(SubCommand::with_name(TransactionCmd::AddOutput.as_string())
            .about("Add an output to a transaction")
            .arg(transaction_argument_name_definition())
            .arg(transaction_argument_address_definition().requires("TRANSACTION_AMOUNT"))
            .arg(transaction_argument_amount_definition())
        )
        .subcommand(SubCommand::with_name(TransactionCmd::RmInput.as_string())
            .about("Remove an input to a transaction")
            .arg(transaction_argument_name_definition())
            .arg(transaction_argument_txid_definition())
            .arg(transaction_argument_index_definition())
        )
        .subcommand(SubCommand::with_name(TransactionCmd::RmOutput.as_string())
            .about("Remove an output to a transaction")
            .arg(transaction_argument_name_definition())
            .arg(transaction_argument_address_definition())
        )
        .subcommand(SubCommand::with_name(TransactionCmd::Status.as_string())
            .about("Status of a staging transaction")
            .arg(transaction_argument_name_definition())
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
