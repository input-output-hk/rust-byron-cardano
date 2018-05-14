use clap::{ArgMatches, App};

pub trait HasCommand {
    type Output;
    type Config;

    fn clap_options<'a, 'b>() -> App<'a, 'b>;

    fn run(cfg: Self::Config, args: &ArgMatches) -> Self::Output;
}
