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
    epoch_id: EpochId,
    start_header_hash: HeaderHash,
    previous_header_hash: HeaderHash,
    upper_bounder_hash: HeaderHash
}
#[derive(Debug)]
pub struct FetchEpochResult {
    last_header_hash: HeaderHash,

    packhash: PackHash
}
