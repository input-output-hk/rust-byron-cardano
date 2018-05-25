use blockchain::{BlockHeader, Block, HeaderHash, EpochId};
use storage::{Storage, types::{PackHash}};

use network::{Result};
use config::{net};

pub trait Api {
    fn get_tip(&mut self) -> Result<BlockHeader>;

    fn get_block(&mut self, hash: HeaderHash) -> Result<Block>;

    fn fetch_epoch(&mut self, config: &net::Config, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult>;
}

#[derive(Debug)]
pub struct FetchEpochParams {
    pub epoch_id: EpochId,
    pub start_header_hash: HeaderHash,
    pub previous_header_hash: HeaderHash,
    pub upper_bound_hash: HeaderHash
}
#[derive(Debug)]
pub struct FetchEpochResult {
    pub last_header_hash: HeaderHash,
    pub previous_last_header_hash: HeaderHash,

    pub packhash: PackHash
}
