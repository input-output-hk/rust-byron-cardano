use cardano::{bip::bip39, wallet};
use command::{HasCommand};
use clap::{ArgMatches, Arg, App};

use super::util::{generate_entropy};
use super::config;

pub struct CommandNewWallet;

impl HasCommand for CommandNewWallet {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "new";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("create a new wallet")
            .arg(Arg::with_name("LANGUAGE")
                .long("language")
                .takes_value(true)
                .value_name("LANGUAGE")
                .possible_values(&["english"])
                .help("use the given language for the mnemonic")
                .required(false)
                .default_value(r"english")
            )
            .arg(Arg::with_name("NO PAPER WALLET")
                .long("no-paper-wallet")
                .takes_value(false)
                .help("if this option is set, the interactive mode won't ask you about generating a paperwallet")
                .required(false)
            )
            .arg(Arg::with_name("MNEMONIC SIZE")
                .long("number-of-mnemonic-words")
                .takes_value(true)
                .value_name("MNEMONIC_SIZE")
                .possible_values(&["12", "15", "18", "21", "24"])
                .help("set the number of the mnemonic words")
                .required(false)
                .default_value(r"15")
            )
            .arg(Arg::with_name("PASSWORD")
                .long("--password")
                .takes_value(true)
                .value_name("PASSWORD")
                .help("set the password from the CLI instead of prompting for it. It is quite unsafe as the password can be visible from your shell history.")
                .required(false)
            )
            .arg(Arg::with_name("EPOCH START")
                .long("--epoch-start")
                .takes_value(true)
                .value_name("EPOCH")
                .help("set the epoch where this wallet was created. if the option is not set then, the network associated with the blockchain is queries and the current stable epoch is set as the start")
                .required(false)
            )
            .arg(Arg::with_name("WALLET NAME").help("the name of the new wallet").index(1).required(true))
            .arg(Arg::with_name("BLOCKCHAIN").help("the name of the associated blockchain (see command `blockchain')").index(2).required(true))
    }
    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        let name        = value_t!(args.value_of("WALLET NAME"), String).unwrap();
        let blockchain  = value_t!(args.value_of("BLOCKCHAIN"), String).unwrap();
        let language    = value_t!(args.value_of("LANGUAGE"), String).unwrap(); // we have a default value
        let mnemonic_sz = value_t!(args.value_of("MNEMONIC SIZE"), bip39::Type).unwrap();
        let password    = value_t!(args.value_of("PASSWORD"), String).ok();
        let epoch_start = value_t!(args.value_of("EPOCH START"), u32).ok();
        let without_paper_wallet = args.is_present("NO PAPER WALLET");
        let seed = generate_entropy(language, password, mnemonic_sz, without_paper_wallet);
        let wallet = wallet::Wallet::new_from_bip39(&seed);

        let config = config::Config::from_wallet(wallet, blockchain, epoch_start);

        config.to_file(&name).unwrap();
    }
}
