use cardano::{util::{hex}, block::{BlockDate, HeaderHash}};
use std::{fmt};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatePtr {
    pub latest_addr: Option<BlockDate>,
    pub latest_known_hash: HeaderHash,
}
impl fmt::Display for StatePtr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(ref bd) = self.latest_addr {
            write!(f, "{}: {}", hex::encode(self.latest_known_hash.as_ref()), bd)
        } else {
            write!(f, "{}: Blockchain's genesis (The BigBang)", hex::encode(self.latest_known_hash.as_ref()))
        }
    }
}
impl StatePtr {
    pub fn new_before_genesis(before_genesis: HeaderHash) -> Self {
        StatePtr { latest_addr: None, latest_known_hash: before_genesis }
    }
    pub fn new(latest_addr: BlockDate, latest_known_hash: HeaderHash) -> Self {
        StatePtr { latest_addr: Some(latest_addr), latest_known_hash }
    }

    pub fn latest_block_date(&self) -> BlockDate {
        if let Some(ref date) = self.latest_addr {
            date.clone()
        } else {
            BlockDate::Genesis(0)
        }
    }
}
