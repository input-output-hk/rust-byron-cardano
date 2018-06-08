use command::{HasCommand};
use clap::{ArgMatches, App};

mod new;
mod recover;
mod address;
mod util;
mod state;

mod config;

pub struct Wallet;

impl HasCommand for Wallet {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "wallet";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("wallet management")
            .subcommand(new::CommandNewWallet::mk_command())
            .subcommand(recover::Recover::mk_command())
            .subcommand(address::Generate::mk_command())
            .subcommand(state::Update::mk_command())
    }
    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        match args.subcommand() {
            (new::CommandNewWallet::COMMAND, Some(opts)) => new::CommandNewWallet::run((), opts),
            (recover::Recover::COMMAND, Some(opts)) => recover::Recover::run((), opts),
            (address::Generate::COMMAND, Some(opts)) => address::Generate::run((), opts),
            (state::Update::COMMAND, Some(opts)) => state::Update::run((), opts),
            _ => {
                println!("{}", args.usage());
                ::std::process::exit(1);
            },
        }
    }
}
