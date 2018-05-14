use command::{HasCommand};
use clap::{ArgMatches, Arg, App};
use config::{Config};

use super::util::{recover_paperwallet, recover_entropy};
use super::Wallet;

pub struct Recover;

impl HasCommand for Recover {
    type Output = Option<Config>;
    type Config = Config;

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
    }
    fn run(mut cfg: Config, args: &ArgMatches) -> Self::Output {
        assert!(cfg.wallet.is_none());
        let language    = value_t!(args.value_of("LANGUAGE"), String).unwrap(); // we have a default value
        let password    = value_t!(args.value_of("PASSWORD"), String).ok();
        let from_paper_wallet = args.is_present("FROM PAPER WALLET");
        let seed = if from_paper_wallet {
            recover_paperwallet(language, password)
        } else {
            recover_entropy(language, password)
        };
        cfg.wallet = Some(Wallet::generate(seed));
        let _storage = cfg.get_storage().unwrap();
        Some(cfg) // we need to update the config's wallet
    }
}