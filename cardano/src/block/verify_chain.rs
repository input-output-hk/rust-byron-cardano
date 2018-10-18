use std::mem;
use std::cmp;
use block::*;
use address;
use config::{GenesisData, ChainParameters, BootStakeholder};
use coin;
use tx::{self, TxAux, TxoPointer, TxOut, TxInWitness};
use std::collections::{BTreeMap, BTreeSet};
use fee::{FeeAlgorithm};
use hash;

pub type Utxos = BTreeMap<TxoPointer, TxOut>;

#[derive(Debug, Clone)]
pub struct ChainState {
    // The current protocol version. All blocks are verified according
    // to the protocol parameters / rules corresponding to
    // adopted_version regardless of the version specified in the
    // block's header. The latter is only used (at the start of a new
    // epoch) to decide whether to adopt a new version (namely if a
    // sufficient number of stakeholders have upgraded).
    pub adopted_version: BlockVersion,

    // The protocol parameters corresponding to adopted_version. These
    // may change when a new version is adopted.
    pub chain_parameters: ChainParameters,

    // Note: currently, stakeholders == boot_stakeholders. FIXME: we
    // may want a map indexed by delegate_pk.
    pub stakeholders: BTreeMap<address::StakeholderId, BootStakeholder>,
    pub total_stake_weight: StakeWeight,

    pub prev_block: HeaderHash,
    pub prev_date: Option<BlockDate>,
    pub slot_leaders: Vec<address::StakeholderId>,
    pub utxos: Utxos,

    // Updates.
    pub active_proposals: BTreeMap<update::UpId, ActiveProposal>,
    pub competing_proposals: BTreeMap<BlockVersion, CompetingProposal>,

    // Some stats.
    pub nr_transactions: u64,
    pub spend_txos: u64,
}

#[derive(Debug, Clone)]
pub struct ActiveProposal {
    pub date: BlockDate,
    pub proposal: update::UpdateProposal,
    pub votes: Vec<(update::UpdateVote, StakeWeight)>,
}

pub type StakeWeight = u64;

#[derive(Debug, Clone)]
pub struct CompetingProposal {
    pub proposal: update::UpdateProposal,

    // This is 'k' blocks after the block where the proposal was
    // accepted.
    pub confirmation_date: BlockDate,

    // Stakeholders that have issued blocks with this version.
    pub issuers: BTreeSet<address::StakeholderId>,
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
            adopted_version: BlockVersion::new(0, 0, 0),
            chain_parameters: genesis_data.chain_parameters.clone(),
            stakeholders: genesis_data.boot_stakeholders.clone(),
            total_stake_weight: genesis_data.boot_stakeholders.iter().map(|(_, v)| v.weight as u64).sum(),
            prev_block: genesis_data.genesis_prev.clone(),
            prev_date: None,
            slot_leaders: vec![],
            utxos,
            active_proposals: BTreeMap::new(),
            competing_proposals: BTreeMap::new(),
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
    pub fn verify_block(&mut self, block_hash: &HeaderHash,
                        blk: &Block, rblk: &RawBlock)
                        -> Result<(), Error>
    {
        let mut res = Ok(());

        let date = blk.get_header().get_blockdate();

        add_error(&mut res, self.do_verify(block_hash, blk, rblk));

        match blk {

            Block::BoundaryBlock(blk) => {
                self.slot_leaders = blk.body.slot_leaders.clone();
                // TODO: check leaders

                // Check whether any competing proposals have reached
                // the adoption threshold.
                let mut new_version = None;

                for (competitor_version, competitor) in &self.competing_proposals {

                    let mut stake_weight: StakeWeight = 0;

                    for issuer in &competitor.issuers {
                        let stakeholder = self.stakeholders.get(issuer).unwrap();
                        stake_weight += stakeholder.weight as StakeWeight;
                    }

                    let threshold = cmp::max(self.chain_parameters.softfork_min_thd,
                                             self.chain_parameters.softfork_init_thd -
                                             (date.get_epochid() - competitor.confirmation_date.get_epochid()) // FIXME: underflow
                                             * self.chain_parameters.softfork_thd_decrement);

                    //debug!("competitor {} has stake {}, threshold is {}", competitor_version, stake_weight, threshold);

                    if (stake_weight as f64) / (self.total_stake_weight as f64) * 1e15 >= threshold as f64 {
                        //debug!("block version {} adopted at {}", competitor_version, date);
                        new_version = Some(competitor_version.clone());
                    }
                }

                if let Some(new_version) = new_version {
                    let competitor = self.competing_proposals.remove(&new_version).unwrap();
                    self.adopt_new_version(&competitor.proposal);
                }
            },

            Block::MainBlock(blk) => {

                // Check the block version: it must be either the
                // currently adopted version or a competing version.
                let block_version = blk.header.extra_data.block_version;
                if let Some(competitor) = self.competing_proposals.get_mut(&block_version) {
                    if date >= competitor.confirmation_date {
                        competitor.issuers.insert(
                            address::StakeholderId::new(&blk.header.consensus.leader_key));
                    } else {
                        add_error(&mut res, Err(Error::WrongBlockVersion(block_version)));
                    }
                } else if block_version != self.adopted_version {
                    add_error(&mut res, Err(Error::WrongBlockVersion(block_version)));
                }

                // Update the utxos from the transactions.
                for txaux in blk.body.tx.iter() {
                    add_error(&mut res, self.verify_tx(txaux));
                }

                // Add update proposals and/or votes.
                add_error(&mut res, self.handle_update(&blk.body.update, &date));
            }
        };

        self.prev_block = block_hash.clone();
        self.prev_date = Some(date);

        res
    }

    fn do_verify(&self, block_hash: &HeaderHash, blk: &Block, rblk: &RawBlock) -> Result<(), Error>
    {
        // Perform stateless checks.
        verify_block(&self.chain_parameters, block_hash, blk, rblk)?;

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
                if date.get_epochid() != (prev_date.get_epochid() + if date.is_boundary() { 1 } else { 0 }) {
                    return Err(Error::BlockDateInFuture)
                }
            }

            None => {
                if date != BlockDate::Boundary(0) { // FIXME: use epoch_start
                    return Err(Error::BlockDateInFuture)
                }
            }
        }

        // Check that the block was signed by the appointed slot leader.
        match blk {

            Block::BoundaryBlock(_) => { },

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

                // TODO: properly check that block_signature is
                // issued/delegated by the slot leader.
            }
        };

        // FIXME: check that block version is currently adopted or competing.

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
                match self.chain_parameters.fee_policy.calculate_for_txaux(&txaux) {
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

    fn handle_update(&mut self, update: &update::UpdatePayload, date: &BlockDate) -> Result<(), Error> {
        let mut res = Ok(());

        if let Some(proposal) = &update.proposal {
            let proposal_id = hash::Blake2b256::new(&cbor!(proposal)?);

            let stakeholder = self.stakeholders.iter().find(|&(_, v)| v.delegate_pk == proposal.from);

            if let Some((_, stakeholder)) = stakeholder {

                // Check that the proposer has sufficient stake.
                // FIXME: investigate exactly how we're supposed to handle precision etc.
                if (stakeholder.weight as f64) / (self.total_stake_weight as f64) * 1e15
                    < self.chain_parameters.update_proposal_thd as f64
                {
                    add_error(&mut res, Err(Error::InsufficientProposerStake));
                }

            } else {
                add_error(&mut res, Err(Error::UnknownProposer));
            }

            self.active_proposals.insert(proposal_id, ActiveProposal {
                date: date.clone(),
                proposal: proposal.clone(),
                votes: vec![],
            });
        }

        for vote in &update.votes {

            // "Voter stake check"
            let stakeholder = self.stakeholders.iter().find(|&(_, v)| v.delegate_pk == vote.key);

            if let Some((_, stakeholder)) = stakeholder {

                // FIXME: investigate exactly how we're supposed to handle precision etc.
                if (stakeholder.weight as f64) / (self.total_stake_weight as f64) * 1e15
                    < self.chain_parameters.update_vote_thd as f64
                {
                    add_error(&mut res, Err(Error::InsufficientVoterStake));
                }

                // "Existence proposal check" and "Active proposal check"
                let mut approved = false;
                let mut rejected = false;

                if let Some(proposal) = self.active_proposals.get_mut(&vote.proposal_id) {
                    // TODO: "Revote check"
                    proposal.votes.push((vote.clone(), stakeholder.weight as StakeWeight));

                    // Check whether the proposal has reached 50% stake in favor or against.
                    // TODO: do proposals ever expire?
                    let mut stake_weight_for = 0;
                    let mut stake_weight_against = 0;
                    for vote in &proposal.votes {
                        if vote.0.decision {
                            stake_weight_for += vote.1;
                        } else {
                            stake_weight_against += vote.1;
                        }
                    }

                    if (stake_weight_for as f64) / (self.total_stake_weight as f64) > 0.5 {
                        approved = true;
                    } else if (stake_weight_against as f64) / (self.total_stake_weight as f64) > 0.5 {
                        rejected = true;
                    }

                } else {
                    add_error(&mut res, Err(Error::MissingProposal));
                }

                if approved {
                    //debug!("proposal {} approved at {}", vote.proposal_id, date);
                    let proposal = self.active_proposals.remove(&vote.proposal_id).unwrap();

                    // Ignore proposals that don't change the
                    // version. Presumably they just deploy a new
                    // software version. TODO: verify that this
                    // proposal's block_version_mod doesn't change any
                    // parameters.
                    if proposal.proposal.block_version != self.adopted_version {
                        self.competing_proposals.insert(
                            proposal.proposal.block_version.clone(),
                            CompetingProposal {
                                proposal: proposal.proposal,
                                confirmation_date: *date /* + k */, // FIXME
                                issuers: BTreeSet::new(),
                            });
                    }
                }

                if rejected {
                    //debug!("proposal {} rejected at {}", vote.proposal_id, date);
                    self.active_proposals.remove(&vote.proposal_id);
                }

            } else {
                add_error(&mut res, Err(Error::UnknownVoter));
            }
        }

        res
    }

    /// Apply any chain parameters changes from the given update
    /// proposal.
    fn adopt_new_version(&mut self, proposal: &update::UpdateProposal) {
        self.adopted_version = proposal.block_version;
        let mods = &proposal.block_version_mod;
        maybe_assign(&mut self.chain_parameters.max_block_size, &mods.max_block_size);
        maybe_assign(&mut self.chain_parameters.max_header_size, &mods.max_header_size);
        maybe_assign(&mut self.chain_parameters.max_tx_size, &mods.max_tx_size);
        maybe_assign(&mut self.chain_parameters.max_proposal_size, &mods.max_proposal_size);
        // TODO
        //maybe_assign(&mut self.chain_parameters.fee_policy, &mods.tx_fee_policy);
        //maybe_assign(&mut self.chain_parameters.update_proposal_thd, &mods.update_proposal_thd);
        //maybe_assign(&mut self.chain_parameters.update_vote_thd, &mods.update_vote_thd);
        // softfork_rule
    }
}

// FIXME: might be nice to return a list of errors. Currently we only
// return the first.
fn add_error(res: &mut Result<(), Error>, err: Result<(), Error>) {
    if res.is_ok() && err.is_err() {
        mem::replace(res, err);
    }
}

fn maybe_assign<T: Clone>(dst: &mut T, src: &Option<T>) {
    if let Some(src) = src {
        mem::replace(dst, src.clone());
    }
}
