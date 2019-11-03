use address;
use block::*;
use coin;
use fee::FeeAlgorithm;
use tx::{TxAux, TxInWitness, TxoPointer};

impl ChainState {
    /// Verify a block in the context of the chain. Regardless of
    /// errors, the chain state is updated to reflect the changes
    /// introduced by this block.
    /// FIXME: we may want to return all errors rather than just the first.
    pub fn verify_block(&mut self, block_hash: &HeaderHash, blk: &Block) -> Result<(), Error> {
        let mut res = Ok(());

        let epoch_transition = self
            .last_date
            .map(|d| d.get_epochid() < blk.header().blockdate().get_epochid())
            .unwrap_or(false);

        if epoch_transition {
            self.slot_leaders = None;
        }

        add_error(&mut res, self.do_verify(block_hash, blk));

        self.last_block = block_hash.clone();
        self.last_date = Some(blk.header().blockdate());
        // FIXME: count boundary blocks as part of the chain length?
        self.chain_length += 1;

        match blk {
            Block::BoundaryBlock(blk) => {
                self.last_boundary_block = Some(block_hash.clone());
                self.slot_leaders = Some(blk.body.slot_leaders.clone());
            }

            Block::MainBlock(_) => {
                if epoch_transition {
                    self.last_boundary_block = Some(block_hash.clone());
                }
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

    fn do_verify(&self, block_hash: &HeaderHash, blk: &Block) -> Result<(), Error> {
        // Perform stateless checks.
        verify_block(block_hash, blk)?;

        // Check the protocol magic.
        if blk.get_protocol_magic() != self.protocol_magic {
            return Err(Error::WrongMagic);
        }

        let hdr = blk.header();
        let prev_block = hdr.previous_header();
        if prev_block != self.last_block {
            return Err(Error::WrongPreviousBlock(
                prev_block,
                self.last_block.clone(),
            ));
        }

        // Check the block date.
        let date = hdr.blockdate();

        match self.last_date {
            Some(last_date) => {
                if date <= last_date {
                    return Err(Error::BlockDateInPast);
                }

                if date.is_boundary() {
                    if date.get_epochid() == last_date.get_epochid() {
                        return Err(Error::BlockDateInPast);
                    } else if date.get_epochid() > last_date.get_epochid() + 1 {
                        return Err(Error::BlockDateInFuture);
                    }
                }
            }

            None => {
                if date != BlockDate::Boundary(0) {
                    // FIXME: use epoch_start
                    return Err(Error::BlockDateInFuture);
                }
            }
        }

        // Check that the block was signed by the appointed slot leader.
        match blk {
            Block::BoundaryBlock(_) => {}

            Block::MainBlock(blk) => {
                let slot_id = blk.header.consensus.slot_id.slotid as usize;

                match &self.slot_leaders {
                    Some(ref slot_leaders) => {
                        if slot_id >= slot_leaders.len() {
                            return Err(Error::NonExistentSlot);
                        }

                        let slot_leader = &slot_leaders[slot_id];
                        // Note: the block signature was already checked in
                        // verify_block, so here we only check the leader key
                        // against the genesis block.
                        if slot_leader
                            != &address::StakeholderId::new(&blk.header.consensus.leader_key)
                        {
                            return Err(Error::WrongSlotLeader);
                        }
                    }
                    None => {}
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
                    self.spent_txos += 1;

                    let witness_address = match in_witness {
                        TxInWitness::PkWitness(pubkey, _) => address::ExtendedAddr::new(
                            address::AddrType::ATPubKey,
                            address::SpendingData::PubKeyASD(*pubkey),
                            txout.address.attributes.clone(),
                        ),

                        TxInWitness::ScriptWitness(_, _) => {
                            panic!("script witnesses are not implemented")
                        }

                        TxInWitness::RedeemWitness(pubkey, _) => {
                            nr_redeems += 1;

                            address::ExtendedAddr::new(
                                address::AddrType::ATRedeem,
                                address::SpendingData::RedeemASD(*pubkey),
                                txout.address.attributes.clone(),
                            )
                        }
                    };

                    if witness_address != txout.address {
                        add_error(&mut res, Err(Error::AddressMismatch));
                    }

                    match input_amount + txout.value {
                        Ok(x) => {
                            input_amount = x;
                        }
                        Err(coin::Error::OutOfBound(_)) => {
                            add_error(&mut res, Err(Error::InputsTooBig))
                        }
                        Err(err) => unreachable!("{}", err),
                    }
                }
            }
        }

        // Calculate the output amount.
        let mut output_amount = coin::Coin::zero();
        for output in &tx.outputs {
            match output_amount + output.value {
                Ok(x) => {
                    output_amount = x;
                }
                Err(coin::Error::OutOfBound(_)) => add_error(&mut res, Err(Error::OutputsTooBig)),
                Err(err) => unreachable!("{}", err),
            }
        }

        // Calculate the minimum fee. The fee is 0 if all inputs are
        // from redeem addresses.
        let min_fee = if nr_redeems == tx.inputs.len() {
            coin::Coin::zero()
        } else {
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
            Err(err) => unreachable!("{}", err),
        };

        // Check that total outputs + minimal fee <= total inputs.
        if output_plus_fee > input_amount {
            add_error(&mut res, Err(Error::OutputsExceedInputs));
        }

        // Add the outputs to the utxo state.
        for (index, output) in tx.outputs.iter().enumerate() {
            if self
                .utxos
                .insert(
                    TxoPointer {
                        id,
                        index: index as u32,
                    },
                    output.clone(),
                )
                .is_some()
            {
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
        *res = err;
    }
}
