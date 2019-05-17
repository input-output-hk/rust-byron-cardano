use super::P2pService;
use crate::{error::Error, subscription::BlockExch};

use chain_core::property::{Block, HasHeader};

use futures::prelude::*;

/// Interface for the blockchain node service responsible for
/// providing access to blocks.
pub trait BlockService: P2pService {
    /// The type of blockchain block served by this service.
    type Block: Block + HasHeader;

    /// The type of asynchronous futures returned by method `tip`.
    ///
    /// The future resolves to the block identifier and the block date
    /// of the current chain tip as known by the serving node.
    type TipFuture: Future<Item = <Self::Block as HasHeader>::Header, Error = Error>;

    fn tip(&mut self) -> Self::TipFuture;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `pull_blocks_to_tip`.
    type PullBlocksToTipStream: Stream<Item = Self::Block, Error = Error>;

    /// The type of asynchronous futures returned by method `pull_blocks_to_tip`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type PullBlocksToTipFuture: Future<Item = Self::PullBlocksToTipStream, Error = Error>;

    fn pull_blocks_to_tip(
        &mut self,
        from: &[<Self::Block as Block>::Id],
    ) -> Self::PullBlocksToTipFuture;

    /// The type of an asynchronous stream that provides blocks in
    /// response to method `get_blocks`.
    type GetBlocksStream: Stream<Item = Self::Block, Error = Error>;

    /// The type of asynchronous futures returned by method `get_blocks`.
    ///
    /// The future resolves to a stream that will be used by the protocol
    /// implementation to produce a server-streamed response.
    type GetBlocksFuture: Future<Item = Self::GetBlocksStream, Error = Error>;

    /// Retrieves the identified blocks in an asynchronous stream.
    fn get_blocks(&mut self, ids: &[<Self::Block as Block>::Id]) -> Self::GetBlocksFuture;

    // The type of an asynchronous stream that provides block headers in
    // response to method `get_headers`.
    //type GetHeadersStream: Stream<Item = <Self::Block as Block>::Header, Error = Error>;

    // The type of asynchronous futures returned by method `get_headers`.
    //
    // The future resolves to a stream that will be used by the protocol
    // implementation to produce a server-streamed response.
    //type GetHeadersFuture: Future<Item = Self::GetHeadersStream, Error = Error>;

    /// The type of asynchronous futures returned by method `block_exchange`.
    ///
    /// The future resolves to a stream of `BlockExch` items sent by
    /// the remote node and the identifier of the node in the network.
    type BlockExchangeFuture: Future<
        Item = (Self::BlockExchangeStream, Self::NodeId),
        Error = Error,
    >;

    /// The sink type for `BlockExch::Solicit` items produced by
    /// `Self::BlockExchangeStream`.
    type BlockSolicitationSink: Sink<SinkItem = Self::Block, SinkError = Error>;

    /// The type of an asynchronous stream that provides notifications
    /// of blocks announced or solicited by the remote node.
    type BlockExchangeStream: Stream<
        Item = BlockExch<Self::Block, Self::BlockSolicitationSink>,
        Error = Error,
    >;

    /// Establishes a bidirectional exchange of blocks
    /// created or accepted by either of the peers.
    ///
    /// The client can use the stream that the returned future resolves to
    /// as a long-lived subscription handle.
    fn block_exchange<Out, SolSink>(&mut self, outbound: Out) -> Self::BlockExchangeFuture
    where
        Out: Stream<Item = BlockExch<Self::Block, SolSink>> + Send + 'static,
        SolSink: Sink<SinkItem = Self::Block> + Send + 'static;
}
