use cardano::{util::base58, wallet::{bip44, scheme::Account}};
use command::{HasCommand};
use clap::{ArgMatches, Arg, App};
use super::util::{create_new_account};

use super::config;

pub struct Generate;

impl HasCommand for Generate {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "address";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("create an address with the given options")
            .arg(Arg::with_name("is_internal").long("internal").help("to generate an internal address (see BIP44)"))
            .arg(Arg::with_name("WALLET NAME").help("the name of the new wallet").index(1).required(true))
            .arg(Arg::with_name("WALLET ACCOUNT").help("account to generate an address in").index(2).required(true))
            .arg(Arg::with_name("indices")
                .help("list of indices for the addresses to create")
                .multiple(true)
            )
    }
    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        let name         = value_t!(args.value_of("WALLET NAME"), String).unwrap();
        let account_name = value_t!(args.value_of("WALLET ACCOUNT"), String).unwrap();
        let addr_type = if args.is_present("is_internal") {
            bip44::AddrType::Internal
        } else {
            bip44::AddrType::External
        };
        let indices = values_t!(args.values_of("indices"), u32).unwrap_or_else(|_| vec![0]);

        let wallet = config::Config::from_file(&name).unwrap();
        let mut known_accounts = config::Accounts::from_files(&name).unwrap();

        let account = known_accounts.get_account_alias(&account_name)
            .or_else(|_| name.parse::<u32>().map_err(|_| config::Error::AccountAliasNotFound(account_name.clone())).and_then(|idx| known_accounts.get_account_index(idx)));
        let account = match account {
            Ok(account) => account,
            Err(config::Error::AccountAliasNotFound(alias)) => {
                let account = create_new_account(&mut known_accounts, &wallet, alias);
                known_accounts.to_files(&name).unwrap();
                account
            },
            Err(err) => {
                error!("error when retrieving an account: {:?}", err);
                panic!()
            }
        };

        let indices : Vec<_> = indices.into_iter().map(|i| (addr_type, i)).collect();
        let addresses = account.generate_addresses(indices.iter());
        for addr in addresses {
            println!("{}", base58::encode(&addr.to_bytes()));
        };
    }
}
