
pub mod lookup;
pub mod randomindex;
pub mod sequentialindex;
pub mod log;

use blockchain::{Block, BlockDate, HeaderHash, SlotId};
use command::{HasCommand};
use clap::{ArgMatches, Arg, App};

use super::config;
use self::log::{LogLock, LogWriter};

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
        let accounts    = config::Accounts::from_files(&wallet_name).unwrap();

        // retrieve the associated blockchain and its storage
        let blockchain_cfg = wallet_cfg.blockchain_config().unwrap();
        let mut storage    = wallet_cfg.blockchain_storage().unwrap();

        // 1. we need to retrieve what is the new tip of the network
        //    so we know when if we need to perform an update and we
        //    can check we have actually reached the tip when we do
        //    the update.
        //
        // let network_tip = storage.get_tip().unwrap();

        // 2. from the wallet_state we need to be able to get the starting point
        //    or actually, the last known state_ptr (BlockDate and Hash);
        //
        // let current_ptr = wallet_state.state_prt()
        let current_ptr = lookup::StatePtr::new_before_genesis(
            blockchain_cfg.genesis_prev
        );

        // 3. we need to be able to retrieve the wallet's lookup structure
        //
        // i.e. we need to know if it is a bip44 or a random address method
        //      for now we assume a bip44 sequential indexing
        let mut lookup_structure = sequentialindex::SequentialBip44Lookup::new(wallet_cfg.wallet().unwrap());
        for _ in accounts.iter() {
           lookup_structure.prepare_next_account().unwrap();
        }

        // 4. try to load the wallet state from the wallet log
        let mut state = lookup::State::load(&wallet_name, current_ptr, lookup_structure).unwrap();

        // 5. perform the lookup now. We may want to save wallet logs as we find them:
        //    - ... non exhausted list of element to log, we will need to look at utxo and co
        //
        //    we also need to update the wallet state on the fly so
        //    we can display something to the user too

        let latest_block_date = state.ptr.latest_block_date();
        let (epoch_start, slot_start) = match &latest_block_date {
            BlockDate::Genesis(epoch) => (*epoch, None),
            BlockDate::Normal(slot)   => (slot.epoch, Some(slot.slotid)),
        };
        let mut iter = storage.iterate_from_epoch(epoch_start).unwrap();
        info!("starting to update wallet state:");
        info!("  from block- {}", latest_block_date);
        info!("  known utxos {:?}", state.utxos);
        debug!("epoch_start: {:?}, slot_start: {:?}", epoch_start, slot_start);
        if slot_start.is_some() || epoch_start > 0 {
            while let Some(blk) = iter.next_block().unwrap() {
                let hdr = blk.get_header();
                debug!("skipping: {}", hdr.get_blockdate());
                if hdr.get_blockdate() >= latest_block_date {
                    break;
                }
            }
        }
        let lock = LogLock::acquire_wallet_log_lock(&wallet_name).unwrap();
        let mut log_writer = log::LogWriter::open(lock).unwrap();
        while let Some(blk) = iter.next_block().unwrap() {
            let events = state.forward(&[blk]).unwrap();
            for ev in events {
                log_writer.append(&ev).unwrap();
            }
        }

        unimplemented!()
    }
}
