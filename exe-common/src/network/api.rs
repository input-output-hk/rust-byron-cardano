use cardano::{block::{Block, BlockHeader, RawBlock, HeaderHash, BlockDate}, tx::{TxAux}};
use network::{Result};

/// Api to abstract the network interaction and do the
/// necessary operations
pub trait Api {
    /// Recover the latest known block header of
    /// a given network
    fn get_tip(&mut self) -> Result<BlockHeader>;

    /// Wait until a new tip is available
    fn wait_for_new_tip(&mut self, prev_tip: &HeaderHash) -> Result<BlockHeader>;

    /// Get one specific block (represented by its unique hash) from the
    /// network
    fn get_block(&mut self, hash: &HeaderHash) -> Result<RawBlock>;

    /// Get the blocks in the half-open interval (from, to] (if
    /// inclusive = false) or [from, to] (if inclusive = true). FIXME:
    /// the inclusive = true case is only needed because the native
    /// protocol doesn't support fetching from the genesis_prev hash.
    fn get_blocks<F>( &mut self
                    , from: &BlockRef
                    , inclusive: bool
                    , to: &BlockRef
                    , got_block: &mut F
                    ) -> Result<()>
        where F: FnMut(&HeaderHash, &Block, &RawBlock) -> ();

    fn send_transaction( &mut self, txaux: TxAux) -> Result<bool>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockRef {
    pub hash: HeaderHash,
    pub date: BlockDate,
    pub parent: HeaderHash, // FIXME: remove
}
