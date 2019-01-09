use address;
use block::*;
use cbor_event::{se, Len};
use chain_core::property::{Block, BlockDate};
use config::{GenesisData, ProtocolMagic};
use fee;
use hash;
use std::collections::BTreeMap;
use std::iter::FromIterator;
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
    pub slot_leaders: Vec<address::StakeholderId>,
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
            slot_leaders: vec![],
            utxos,
            chain_length: 0,
            nr_transactions: 0,
            spent_txos: 0,
        }
    }
}

impl chain_core::property::ChainState for ChainState {
    type Block = super::Block;
    type Error = Error;
    type GenesisData = GenesisData;

    fn new(genesis_data: &Self::GenesisData) -> Result<Self, Self::Error> {
        Ok(ChainState::new(&genesis_data))
    }

    fn apply_block(&mut self, block: &Self::Block) -> Result<(), Self::Error> {
        self.verify_block(&block.id(), block)
    }

    fn get_last_block_id(&self) -> HeaderHash {
        self.last_block.clone()
    }

    fn get_chain_length(&self) -> u64 {
        self.chain_length
    }

    type Delta = ClassicChainStateDelta;

    fn diff(from: &Self, to: &Self) -> Result<Self::Delta, Self::Error> {
        assert_ne!(from, to);

        let (removed_utxos, added_utxos) =
            super::super::util::diff_maps::diff_maps(&from.utxos, &to.utxos);

        Ok(ClassicChainStateDelta {
            base: from.last_block.clone(),
            last_block: to.last_block.clone(),
            last_date: to.last_date.unwrap().clone(),
            last_boundary_block: to.last_boundary_block.clone().unwrap(),
            slot_leaders: to.slot_leaders.clone(),
            chain_length: to.chain_length,
            nr_transactions: to.nr_transactions,
            spent_txos: to.spent_txos,
            removed_utxos: removed_utxos.into_iter().map(|x| x.clone()).collect(),
            added_utxos: Utxos::from_iter(
                added_utxos.into_iter().map(|(n, v)| (n.clone(), v.clone())))
        })
    }

    fn apply_delta(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
        assert_eq!(self.last_block, delta.base);
        self.last_block = delta.last_block;
        self.last_date = Some(delta.last_date);
        self.last_boundary_block = Some(delta.last_boundary_block);
        self.chain_length = delta.chain_length;
        self.nr_transactions = delta.nr_transactions;
        self.spent_txos = delta.spent_txos;
        self.slot_leaders = delta.slot_leaders;

        for txo_ptr in &delta.removed_utxos {
            if self.utxos.remove(txo_ptr).is_none() {
                panic!("chain state delta removes non-existent utxo {}", txo_ptr);
            }
        }

        for (txo_ptr, txo) in delta.added_utxos {
            if self.utxos.insert(txo_ptr, txo).is_some() {
                panic!("chain state delta inserts duplicate utxo");
            }
        }

        Ok(())
    }
}

pub struct ClassicChainStateDelta {
    base: HeaderHash,
    last_block: HeaderHash,
    last_date: super::BlockDate,
    last_boundary_block: HeaderHash,
    chain_length: u64,
    nr_transactions: u64,
    spent_txos: u64,
    slot_leaders: Vec<address::StakeholderId>, // FIXME: get from last_boundary_block
    removed_utxos: Vec<TxoPointer>,
    added_utxos: Utxos,
}

const NR_FIELDS: u64 = 10;

impl chain_core::property::ChainStateDelta for ClassicChainStateDelta {
}

impl chain_core::property::Serializable for ClassicChainStateDelta {
    type Error = cbor_event::Error;

    fn serialize<W: std::io::Write>(&self, mut writer: W) -> Result<(), Self::Error> {
        let mut data = vec![];
        {
            let mut serializer = se::Serializer::new(&mut data);
            serializer
                .write_array(Len::Len(NR_FIELDS))?
                .serialize(&self.base)?
                .serialize(&self.last_block)?
                .serialize(&self.last_date.serialize())?
                .serialize(&self.last_boundary_block)?
                .serialize(&self.chain_length)?
                .serialize(&self.nr_transactions)?
                .serialize(&self.spent_txos)?;
            se::serialize_fixed_array(self.slot_leaders.iter(), &mut serializer)?;
            se::serialize_fixed_array(self.removed_utxos.iter(), &mut serializer)?;
            se::serialize_fixed_map(self.added_utxos.iter(), &mut serializer)?;
        }
        writer.write(&data)?;
        Ok(())
    }

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        let mut raw = cbor_event::de::Deserializer::from(reader);

        raw.tuple(NR_FIELDS, "chain state delta")?;
        let base = raw.deserialize()?;
        let last_block = raw.deserialize()?;
        let last_date = BlockDate::deserialize(raw.deserialize()?);
        let last_boundary_block = raw.deserialize()?;
        let chain_length = raw.deserialize()?;
        let nr_transactions = raw.deserialize()?;
        let spent_txos = raw.deserialize()?;
        let slot_leaders = raw.deserialize()?;
        let removed_utxos = raw.deserialize()?;
        let added_utxos = raw.deserialize()?;

        Ok(Self {
            base,
            last_block,
            last_date,
            last_boundary_block,
            slot_leaders,
            chain_length,
            nr_transactions,
            spent_txos,
            removed_utxos,
            added_utxos
        })
    }
}
