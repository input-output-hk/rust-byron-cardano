use std::mem;
use block::*;
use address;
use config::{ProtocolMagic, GenesisData};
use coin;
use tx::{self, TxAux, TxoPointer, TxOut, TxInWitness};
use std::collections::BTreeMap;
use fee::{self, FeeAlgorithm};

pub type Utxos = BTreeMap<TxoPointer, TxOut>;

#[derive(Debug, Clone)]
pub struct ChainState {
    // FIXME: maybe we should just keep a ref to GenesisData?  Though
    // I guess at least the fee policy could change in an update.
    pub protocol_magic: ProtocolMagic,
    pub fee_policy: fee::LinearFee,

    pub prev_block: HeaderHash,
    pub prev_date: Option<BlockDate>,
    pub slot_leaders: Vec<address::StakeholderId>,
    pub utxos: Utxos,

    // Some stats.
    pub nr_transactions: u64,
    pub spend_txos: u64,
}

impl ChainState {

    /// Initialize the initial chain state from the genesis data.
    pub fn new(genesis_data: &GenesisData) -> Self {

        let mut utxos = BTreeMap::new();

        // Create utxos from AVVM distributions.
        for (&pubkey, &value) in genesis_data.avvm_distr.iter() {
            let (id, address) = tx::redeem_pubkey_to_txid(&pubkey);
            utxos.insert(
                TxoPointer { id, index: 0 },
                TxOut { address, value });
        }

        // FIXME: implement non_avvm_balances.

        ChainState {
            protocol_magic: genesis_data.protocol_magic,
            fee_policy: genesis_data.fee_policy,
            prev_block: genesis_data.genesis_prev.clone(),
            prev_date: None,
            slot_leaders: vec![],
            utxos,
            nr_transactions: 0,
            spend_txos: 0,
        }
    }

    /// Initialize the chain state at the start of an epoch from the
    /// utxo state at the end of the previous epoch, and the last
    /// block hash / block date in that epoch.
    pub fn new_from_epoch_start(
        genesis_data: &GenesisData,
        last_block: HeaderHash,
        last_date: BlockDate,
        utxos: Utxos) -> Self
    {
        let mut chain_state = ChainState::new(genesis_data);
        chain_state.prev_block = last_block;
        chain_state.prev_date = Some(last_date);
        chain_state.utxos = utxos;
        chain_state
    }

    /// Verify a block in the context of the chain. Regardless of
    /// errors, the chain state is updated to reflect the changes
    /// introduced by this block.
    /// FIXME: we may want to return all errors rather than just the first.
    pub fn verify_block(&mut self, block_hash: &HeaderHash, blk: &Block) -> Result<(), Error> {

        let mut res = Ok(());

        add_error(&mut res, self.do_verify(block_hash, blk));

        self.prev_block = block_hash.clone();
        self.prev_date = Some(blk.get_header().get_blockdate());

        match blk {

            Block::GenesisBlock(blk) => {
                self.slot_leaders = blk.body.slot_leaders.clone();
            },

            Block::MainBlock(_) => {
            }
        };

        // Update the utxos from the transactions.
        if let Block::MainBlock(blk) = blk {
            for txaux in blk.body.tx.iter() {
                add_error(&mut res, self.verify_tx(txaux));
            }
        }

        res
    }

    fn do_verify(&self, block_hash: &HeaderHash, blk: &Block) -> Result<(), Error>
    {
        // Perform stateless checks.
        verify_block(self.protocol_magic, block_hash, blk)?;

        if blk.get_header().get_previous_header() != self.prev_block {
            return Err(Error::WrongPreviousBlock)
        }

        // Check the block date.
        let date = blk.get_header().get_blockdate();

        match self.prev_date {
            Some(prev_date) => {
                if date <= prev_date {
                    return Err(Error::BlockDateInPast)
                }

                // If this is a genesis block, it should be the next
                // epoch; otherwise it should be in the current epoch.
                if date.get_epochid() != (prev_date.get_epochid() + if date.is_genesis() { 1 } else { 0 }) {
                    return Err(Error::BlockDateInFuture)
                }
            }

            None => {
                if date != BlockDate::Genesis(0) { // FIXME: use epoch_start
                    return Err(Error::BlockDateInFuture)
                }
            }
        }

        // Check that the block was signed by the appointed slot leader.
        match blk {

            Block::GenesisBlock(_) => { },

            Block::MainBlock(blk) => {
                let slot_id = blk.header.consensus.slot_id.slotid as usize;

                if slot_id >= self.slot_leaders.len() {
                    return Err(Error::NonExistentSlot)
                }

                let slot_leader = &self.slot_leaders[slot_id];

                // Note: the block signature was already checked in
                // verify_block, so here we only check the leader key
                // against the genesis block.
                if slot_leader != &address::StakeholderId::new(&blk.header.consensus.leader_key) {
                    return Err(Error::WrongSlotLeader)
                }
            }
        };

        Ok(())
    }

    /// Verify that a transaction only spends unspent transaction
    /// outputs (utxos), and update the utxo state.
    fn verify_tx(&mut self, txaux: &TxAux) -> Result<(), Error> {

        self.nr_transactions += 1;

        let mut res = Ok(());
        let tx = &txaux.tx;
        let id = tx.id();

        // Look up the utxos corresponding to the inputs and remove
        // them from the utxo map to prevent double spending. Also
        // check that the utxo address matches the witness
        // (i.e. that the witness is actually authorized to spend
        // this utxo).
        // Note: inputs/witnesses size mismatches are detected in
        // verify::verify_block().
        let mut input_amount = coin::Coin::zero();
        let mut nr_redeems = 0;
        for (txin, in_witness) in tx.inputs.iter().zip(txaux.witness.iter()) {
            match self.utxos.remove(&txin) {
                None => {
                    add_error(&mut res, Err(Error::MissingUtxo));
                }
                Some(txout) => {

                    self.spend_txos += 1;

                    let witness_address = match in_witness {

                        TxInWitness::PkWitness(pubkey, _) => {
                            address::ExtendedAddr::new(
                                address::AddrType::ATPubKey,
                                address::SpendingData::PubKeyASD(*pubkey),
                                txout.address.attributes.clone())
                        }

                        TxInWitness::ScriptWitness(_, _) => {
                            panic!("script witnesses are not implemented")
                        }

                        TxInWitness::RedeemWitness(pubkey, _) => {
                            nr_redeems += 1;

                            address::ExtendedAddr::new(
                                address::AddrType::ATRedeem,
                                address::SpendingData::RedeemASD(*pubkey),
                                txout.address.attributes.clone())
                        }
                    };

                    if witness_address != txout.address {
                        add_error(&mut res, Err(Error::AddressMismatch));
                    }

                    match input_amount + txout.value {
                        Ok(x) => { input_amount = x; }
                        Err(coin::Error::OutOfBound(_)) => add_error(&mut res, Err(Error::InputsTooBig)),
                        Err(err) => unreachable!("{}", err)
                    }
                }
            }
        }

        // Calculate the output amount.
        let mut output_amount = coin::Coin::zero();
        for output in &tx.outputs {
            match output_amount + output.value {
                Ok(x) => { output_amount = x; }
                Err(coin::Error::OutOfBound(_)) => add_error(&mut res, Err(Error::OutputsTooBig)),
                Err(err) => unreachable!("{}", err)
            }
        }

        // Calculate the minimum fee. The fee is 0 if all inputs are
        // from redeem addresses.
        let min_fee =
            if nr_redeems == tx.inputs.len() { coin::Coin::zero() }
            else {
                match self.fee_policy.calculate_for_txaux(&txaux) {
                    Ok(fee) => fee.to_coin(),
                    Err(err) => {
                        add_error(&mut res, Err(Error::FeeError(err)));
                        coin::Coin::zero()
                    }
                }
            };

        let output_plus_fee = match output_amount + min_fee {
            Ok(x) => x,
            Err(coin::Error::OutOfBound(_)) => {
                add_error(&mut res, Err(Error::OutputsTooBig));
                output_amount
            }
            Err(err) => unreachable!("{}", err)
        };

        // Check that total outputs + minimal fee <= total inputs.
        if output_plus_fee > input_amount {
            add_error(&mut res, Err(Error::OutputsExceedInputs));
        }

        // Add the outputs to the utxo state.
        for (index, output) in tx.outputs.iter().enumerate() {
            if self.utxos.insert(TxoPointer { id, index: index as u32 }, output.clone()).is_some() {
                add_error(&mut res, Err(Error::DuplicateTxo));
            }
        }

        res
    }
}

// FIXME: might be nice to return a list of errors. Currently we only
// return the first.
fn add_error(res: &mut Result<(), Error>, err: Result<(), Error>) {
    if res.is_ok() && err.is_err() {
        mem::replace(res, err);
    }
}
