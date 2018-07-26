use cardano::block::{Block, BlockHeader, RawBlock, HeaderHash, BlockDate};
use network::{Result};

/// Api to abstract the network interaction and do the
/// necessary operations
pub trait Api {
    /// Recover the latest known block header of
    /// a given network
    fn get_tip(&mut self) -> Result<BlockHeader>;

    /// Get one specific block (represented by its unique hash) from the
    /// network
    fn get_block(&mut self, hash: &HeaderHash) -> Result<RawBlock>;

    /// Get the blocks in the half-open interval (from, to] (if
    /// inclusive = false) or [from, to] (if inclusive = true). FIXME:
    /// the inclusive = true case is only needed because the native
    /// protocol doesn't support fetching from the genesis_prev hash.
    fn get_blocks(&mut self, from: &BlockRef, inclusive: bool, to: &BlockRef,
                   got_block: &mut FnMut(&HeaderHash, &Block, &RawBlock) -> ());
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockRef {
    pub hash: HeaderHash,
    pub date: BlockDate,
    pub parent: HeaderHash, // FIXME: remove
}
