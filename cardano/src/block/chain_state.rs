use address;
use block::*;
use config::{GenesisData, ProtocolMagic};
use fee;
use hash;
use std::collections::BTreeMap;
use tx::{self, TxOut, TxoPointer};

pub type Utxos = BTreeMap<TxoPointer, TxOut>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ChainState {
    // FIXME: maybe we should just keep a ref to GenesisData?  Though
    // I guess at least the fee policy could change in an update.
    pub protocol_magic: ProtocolMagic,
    pub fee_policy: fee::LinearFee,

    pub last_block: HeaderHash,
    pub last_date: Option<super::BlockDate>,
    pub last_boundary_block: Option<HeaderHash>,
    pub slot_leaders: Option<Vec<address::StakeholderId>>,
    pub utxos: Utxos,
    pub chain_length: u64,

    // Some stats.
    pub nr_transactions: u64,
    pub spent_txos: u64,
}

impl ChainState {
    /// Initialize the initial chain state from the genesis data.
    pub fn new(genesis_data: &GenesisData) -> Self {
        let mut utxos = BTreeMap::new();

        // Create utxos from AVVM distributions.
        for (pubkey, value) in &genesis_data.avvm_distr {
            let (id, address) = tx::redeem_pubkey_to_txid(&pubkey, genesis_data.protocol_magic);
            utxos.insert(
                TxoPointer { id, index: 0 },
                TxOut {
                    address,
                    value: value.clone(),
                },
            );
        }

        // Create utxos from non-AVVM balances.
        for (address, value) in &genesis_data.non_avvm_balances {
            let id = hash::Blake2b256::new(&cbor!(&address).unwrap());
            utxos.insert(
                TxoPointer { id, index: 0 },
                TxOut {
                    address: address.deconstruct(),
                    value: value.clone(),
                },
            );
        }

        ChainState {
            protocol_magic: genesis_data.protocol_magic,
            fee_policy: genesis_data.fee_policy,
            last_block: genesis_data.genesis_prev.clone(),
            last_date: None,
            last_boundary_block: None,
            slot_leaders: None,
            utxos,
            chain_length: 0,
            nr_transactions: 0,
            spent_txos: 0,
        }
    }
}
