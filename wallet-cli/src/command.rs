use clap::{ArgMatches, App};

use config::Config;

pub trait HasCommand {
    type Output;

    fn clap_options<'a, 'b>() -> App<'a, 'b>;

    fn run(cfg: Config, args: &ArgMatches) -> Self::Output;
}
