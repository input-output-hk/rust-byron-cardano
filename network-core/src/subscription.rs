use chain_core::property::{Block, HasHeader};

use futures::sink::Sink;

use std::fmt::{self, Debug};

/// An item in a block exchange stream.
pub enum BlockExch<B, S>
where
    B: Block + HasHeader,
    S: Sink<SinkItem = B>,
{
    /// Announcement of a newly minted block.
    Announce(<B as HasHeader>::Header),
    /// Request to send blocks with the identifiers specified in the first
    /// tuple item.
    /// The second tuple item is a `Sink` which can be used on the recipient
    /// side to send the blocks, and on the sender side to forward blocks
    /// received from the network.
    Solicit(Vec<<B as Block>::Id>, S),
}

impl<B, S> Debug for BlockExch<B, S>
where
    B: Block + HasHeader,
    S: Sink<SinkItem = B>,
    <B as HasHeader>::Header: Debug,
    <B as Block>::Id: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlockExch::Announce(header) => f.debug_tuple("Announce").field(header).finish(),
            BlockExch::Solicit(ids, _) => f
                .debug_tuple("Solicit")
                .field(ids)
                .field(&format_args!("_"))
                .finish(),
        }
    }
}
