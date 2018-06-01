
pub mod lookup;
pub mod randomindex;
pub mod sequentialindex;

use command::{HasCommand};
use clap::{ArgMatches, Arg, App};

use super::config;

pub struct Update;

impl HasCommand for Update {
    type Output = ();
    type Config = ();

    const COMMAND : &'static str = "update";

    fn clap_options<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.about("update the state of the given wallet against its configured blockchain")
            .arg(Arg::with_name("WALLET NAME").help("the name of the new wallet").index(1).required(true))
    }
    fn run(_: Self::Config, args: &ArgMatches) -> Self::Output {
        // retrieve user's wallet
        let wallet_name = value_t!(args.value_of("WALLET NAME"), String).unwrap();
        let wallet_cfg  = config::Config::from_file(&wallet_name).unwrap();

        // retrieve the associated blockchain and its storage
        let blockchain_cfg = wallet_cfg.blockchain_config().unwrap();
        let mut storage    = wallet_cfg.blockchain_storage().unwrap();

        // 1. we need to find out what was the last state of the wallet:
        //    TODO: load the wallet logs and fold the state to retrieve where
        //          we need to start from
        //
        // let wallet_state = WalletState::load(&wallet_name).unwrap();

        // 2. we need to retrieve what is the new tip of the network
        //    so we know when if we need to perform an update and we
        //    can check we have actually reached the tip when we do
        //    the update.
        //
        // let network_tip = storage.get_tip().unwrap();

        // 3. perform the lookup now. We may want to save wallet logs as we find them:
        //    - so we know when to start from again (so we need checkpoint or something similar)
        //    - ... non exhausted list of element to log, we will need to look at utxo and co
        //
        //    we also need to update the wallet state on the fly so
        //    we can display something to the user too
        unimplemented!()
    }
}
