use block::*;
use address;
use config::{ProtocolMagic};

pub struct ChainState {
    pub prev_block: HeaderHash,
    pub prev_date: Option<BlockDate>,
    pub slot_leaders: Vec<address::StakeholderId>,
}

impl ChainState {
    pub fn new(genesis_prev: &HeaderHash) -> Self {
        ChainState {
            prev_block: genesis_prev.clone(),
            prev_date: None,
            slot_leaders: vec![]
        }
    }
}

pub fn verify_block_in_chain(
    protocol_magic: ProtocolMagic,
    chain_state: &mut ChainState,
    block_hash: &HeaderHash,
    blk: &Block) -> Result<(), Error>
{
    let res = do_verify(protocol_magic, chain_state, block_hash, blk);

    chain_state.prev_block = block_hash.clone();
    chain_state.prev_date = Some(blk.get_header().get_blockdate());

    match blk {

        Block::GenesisBlock(blk) => {
            chain_state.slot_leaders = blk.body.slot_leaders.clone();
        },

        Block::MainBlock(_) => {
        }
    };

    res
}

fn do_verify(
    protocol_magic: ProtocolMagic,
    chain_state: &mut ChainState,
    block_hash: &HeaderHash,
    blk: &Block) -> Result<(), Error>
{
    verify_block(protocol_magic, block_hash, blk)?;

    if blk.get_header().get_previous_header() != chain_state.prev_block {
        return Err(Error::WrongPreviousBlock)
    }

    // Check the block date.
    let date = blk.get_header().get_blockdate();

    match chain_state.prev_date {
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

            if slot_id >= chain_state.slot_leaders.len() {
                return Err(Error::NonExistentSlot)
            }

            let slot_leader = &chain_state.slot_leaders[slot_id];

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
