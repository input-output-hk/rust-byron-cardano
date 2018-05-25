use blockchain::{BlockHeader, Block, HeaderHash};
use storage::{Storage, types::{PackHash}};

use network::{Result};

pub trait Api {
    fn get_tip(&mut self) -> Result<BlockHeader>;

    fn get_block(&mut self) -> Result<Block>;

    fn fetch_epoch(&mut self, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult>;
}

#[derive(Debug)]
pub struct FetchEpochParams {
    previous_header_hash: HeaderHash
}
#[derive(Debug)]
pub struct FetchEpochResult {
    last_header_hash: HeaderHash,

    packhash: PackHash
}
