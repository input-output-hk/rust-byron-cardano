mod blockchain;
mod block;
mod wallet;

pub use self::blockchain::{*};
pub use self::block::{*};
pub use self::wallet::{*}; 

use clap::{ArgMatches, App, SubCommand};

pub trait HasCommand {
    type Output;
    type Config;

    const COMMAND : &'static str;

    /// returns the subcommand option handling this command
    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b>;

    fn mk_command<'a, 'b>() -> App<'a, 'b> {
        Self::clap_options(SubCommand::with_name(Self::COMMAND))
    }

    fn run(cfg: Self::Config, args: &ArgMatches) -> Self::Output;
}