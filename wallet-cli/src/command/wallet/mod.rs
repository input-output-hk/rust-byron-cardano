use command::{HasCommand};
use clap::{ArgMatches, App};
use config::{Config};

mod definition;
mod new;
mod recover;
mod address;
mod find_address;
mod util;

mod config;

use self::find_address::{FindAddress};
pub use self::definition::{Wallet};

impl HasCommand for Wallet {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "wallet";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("wallet management")
            .subcommand(new::CommandNewWallet::mk_command())
            .subcommand(recover::Recover::mk_command())
            .subcommand(address::Generate::mk_command())
            // TODO: move this command to the blockchain
            // .subcommand(FindAddress::mk_command())
    }
    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        match args.subcommand() {
            (new::CommandNewWallet::COMMAND, Some(opts)) => new::CommandNewWallet::run((), opts),
            (recover::Recover::COMMAND, Some(opts)) => recover::Recover::run((), opts),
            (address::Generate::COMMAND, Some(opts)) => address::Generate::run((), opts),
            /*
            (FindAddress::COMMAND, Some(opts)) => {
                FindAddress::run((), opts);
                None
            },
            */
            _ => {
                println!("{}", args.usage());
                ::std::process::exit(1);
            },
        }
    }
}
