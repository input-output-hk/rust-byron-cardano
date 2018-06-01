
pub mod lookup;
pub mod randomindex;
pub mod sequentialindex;

use blockchain::{Block, BlockDate, HeaderHash, SlotId};
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

        // 3. from the wallet_state we need to be able to get the starting point
        //    or actually, the last known state_ptr (BlockDate and Hash);
        //
        // let current_ptr = wallet_state.state_prt()
        let current_ptr = lookup::StatePtr::new_before_genesis(
            blockchain_cfg.genesis_prev
        );

        // 4. wallet current utxos
        //    we need to be able to retrieve them from the wallet
        //
        // let current_utxos = wallet_state.utxos().clone();
        let current_utxos = lookup::Utxos::new();

        // 5. we need to be able to retrieve the wallet's lookup structure
        //
        // i.e. we need to know if it is a bip44 or a random address method
        //      for now we assume a bip44 sequential indexing
        let lookup_structure = sequentialindex::SequentialBip44Lookup::new(wallet_cfg.wallet().unwrap());

        // 6. construct the lookup state from the current_ptr
        let mut state = lookup::State::new(current_ptr, lookup_structure, current_utxos);

        // 7. perform the lookup now. We may want to save wallet logs as we find them:
        //    - so we know when to start from again (so we need checkpoint or something similar)
        //    - ... non exhausted list of element to log, we will need to look at utxo and co
        //
        //    we also need to update the wallet state on the fly so
        //    we can display something to the user too

        let mut iter = storage.iterate_from_epoch(0).unwrap();
        while let Some(blk) = iter.next_block().unwrap() {
            state.forward(&[blk]).unwrap();
        }

        unimplemented!()
    }
}
