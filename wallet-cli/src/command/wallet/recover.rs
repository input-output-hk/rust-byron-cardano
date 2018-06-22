use command::{HasCommand};
use clap::{ArgMatches, Arg, App};

use super::util::{recover_paperwallet, recover_entropy};
use super::config;
use cardano::wallet;

pub struct Recover;

impl HasCommand for Recover {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "recover";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("recover a wallet from bip39 mnemonics")
            .arg(Arg::with_name("LANGUAGE")
                .long("language")
                .takes_value(true)
                .value_name("LANGUAGE")
                .possible_values(&["english"])
                .help("use the given language for the mnemonic")
                .required(false)
                .default_value(r"english")
            )
            .arg(Arg::with_name("FROM PAPER WALLET")
                .long("from-paper-wallet")
                .takes_value(false)
                .help("if this option is set, we will try to recover the wallet from the paper wallet instead.")
                .required(false)
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
                .help("set the epoch where this wallet was created. if this is not set, the default epoch of 0 is assumed")
                .required(false)
            )
            .arg(Arg::with_name("WALLET NAME").help("the name of the new wallet").index(1).required(true))
            .arg(Arg::with_name("BLOCKCHAIN").help("the name of the associated blockchain (see command `blockchain')").index(2).required(true))
    }
    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        let name        = value_t!(args.value_of("WALLET NAME"), String).unwrap();
        let blockchain  = value_t!(args.value_of("BLOCKCHAIN"), String).unwrap();
        let language    = value_t!(args.value_of("LANGUAGE"), String).unwrap(); // we have a default value
        let password    = value_t!(args.value_of("PASSWORD"), String).ok();
        let epoch_start = value_t!(args.value_of("EPOCH START"), u32).ok();
        let from_paper_wallet = args.is_present("FROM PAPER WALLET");
        let seed = if from_paper_wallet {
            recover_paperwallet(language, password)
        } else {
            recover_entropy(language, password)
        };
        let wallet = wallet::Wallet::new_from_bip39(&seed);

        let config = config::Config::from_wallet(wallet, blockchain, epoch_start);

        config.to_file(&name).unwrap();
    }
}
