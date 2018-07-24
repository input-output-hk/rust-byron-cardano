use cardano::block::{BlockHeader, RawBlock, HeaderHash, EpochId};
use config::{net};
use network::{Result};
use storage::{Storage, types::{PackHash}};

/// Api to abstract the network interaction and do the
/// necessary operations
pub trait Api {
    /// Recover the latest known block header of
    /// a given network
    fn get_tip(&mut self) -> Result<BlockHeader>;

    /// Get one specific block (represented by its unique hash) from the
    /// network
    fn get_block(&mut self, hash: HeaderHash) -> Result<RawBlock>;

    /// Get the blocks in the half-open interval (from, to].
    fn get_blocks(&mut self, from: HeaderHash, to: HeaderHash) -> Result<Vec<(HeaderHash, RawBlock)>>;

    /// Fetch a finished epoch
    ///
    /// Note that calling this api too close to the windows of block instability will either likely
    /// result in failure to get the pack, or unexpected result.
    ///
    /// In the case of hermes, an epoch is pack only after the latest block of the epoch
    /// is considered immutable in the chain by being old enough; latest known block's date
    /// is tip.date() - k where k is typically around 2200 blocks. TODO this is a configurable
    /// parameter, so expect to be able to find the constant somewhere in the net::Config in the future.
    fn fetch_epoch(&mut self, config: &net::Config, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult>;
}

/// Parameter for fetching a full epoch
///
/// the epoch_id refer to which full epoch we want to fetch,
/// which is either used to make sure we're getting the
/// right blocks in the native API, or directly as a way
/// to get the right epoch in the http API.
///
/// The upper bound hash is used for the native API to have
/// a way to query the blocks in a range. we always know
/// the genesis hash (coming from the configuration) and
/// the get_tip API (coming from querying a network node.
///
/// The previous_header_hash is the hash we used to make
/// sure that the chain is actually valid; this hash
/// which represent the latest known block in the previous epoch,
/// should be equal to the genesis block of the new epoch
/// that is going to be fetch
///
/// start_header_hash is only used in the native protocol
/// when downloading the first block of the chain,
/// since we can't start downloading at previous_header_hash
/// which point to a non valid block
#[derive(Debug)]
pub struct FetchEpochParams {
    pub epoch_id: EpochId,
    pub start_header_hash: HeaderHash,
    pub previous_header_hash: HeaderHash,
    pub upper_bound_hash: HeaderHash
}

/// Result values of an epoch fetch.
//
/// it will return:
///
/// * the hash of the pack where the data have been stored,
/// * the hash of the last header fetched
/// * optionally if known, the hash of the next epoch first block
#[derive(Debug)]
pub struct FetchEpochResult {
    pub next_epoch_hash: Option<HeaderHash>,
    pub last_header_hash: HeaderHash,
    pub packhash: PackHash
}
