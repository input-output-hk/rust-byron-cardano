use wallet_crypto::{bip44,};
use wallet_crypto::util::base58;
use command::{HasCommand};
use clap::{ArgMatches, Arg, App};
use config::{Config};
use account::{Account};

pub struct Generate;

impl HasCommand for Generate {
    type Output = Option<Config>;
    type Config = Config;

    const COMMAND : &'static str = "address";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("create an address with the given options")
            .arg(Arg::with_name("is_internal").long("internal").help("to generate an internal address (see BIP44)"))
            .arg(Arg::with_name("account").help("account to generate an address in").index(1).required(true))
            .arg(Arg::with_name("indices")
                .help("list of indices for the addresses to create")
                .multiple(true)
            )
    }
    fn run(cfg: Config, args: &ArgMatches) -> Self::Output {
        match &cfg.wallet {
            &None => panic!("No wallet created, see `wallet generate` command"),
            &Some(ref wallet) => {
                let addr_type = if args.is_present("is_internal") {
                    bip44::AddrType::Internal
                } else {
                    bip44::AddrType::External
                };
                let account_name = args.value_of("account")
                    .and_then(|s| Some(Account::new(s.to_string())))
                    .unwrap();
                let account = match cfg.find_account(&account_name) {
                    None => panic!("no account {:?}", account_name),
                    Some(r) => r,
                };
                let indices = values_t!(args.values_of("indices"), u32).unwrap_or_else(|_| vec![0]);

                let addresses = wallet.0.gen_addresses(account, addr_type, indices).unwrap();
                for addr in addresses {
                    println!("{}", base58::encode(&addr.to_bytes()));
                };
                None // we don't need to update the wallet
            }
        }
    }
}