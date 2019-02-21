use chain_core::property;

use std::fmt::Debug;

/// Traits required for the blockchain block payloads used by the protocol.
pub trait ProtocolBlock:
    property::Block + property::HasHeader + cbor_event::Deserialize + cbor_event::Serialize + Debug
{
}

impl<T> ProtocolBlock for T
where
    T: Debug,
    T: property::Block,
    T: property::HasHeader,
    T: cbor_event::Deserialize + cbor_event::Serialize,
    <T as property::Block>::Id: ProtocolBlockId,
    <T as property::Block>::Date: ProtocolBlockDate,
    <T as property::HasHeader>::Header: ProtocolHeader,
{
}

/// Traits required for the blockchain header payloads used by the protocol.
pub trait ProtocolHeader:
    property::Header + cbor_event::Deserialize + cbor_event::Serialize + Debug
{
}

impl<T> ProtocolHeader for T
where
    T: Debug,
    T: property::Header,
    T: cbor_event::Deserialize + cbor_event::Serialize,
    <T as property::Header>::Id: ProtocolBlockId,
    <T as property::Header>::Date: ProtocolBlockDate,
{
}

/// Traits required for the block id values used by the protocol.
pub trait ProtocolBlockId:
    property::BlockId + cbor_event::Deserialize + cbor_event::Serialize + Debug
{
}

impl<T> ProtocolBlockId for T
where
    T: property::BlockId + Debug,
    T: cbor_event::Deserialize + cbor_event::Serialize,
{
}

/// Traits required for the block date values used by the protocol.
pub trait ProtocolBlockDate: property::BlockDate + Debug {}

impl<T> ProtocolBlockDate for T where T: property::BlockDate + Debug {}

/// Traits required for the transaction id values used by the protocol.
pub trait ProtocolTransactionId:
    property::TransactionId + cbor_event::Deserialize + cbor_event::Serialize + Debug
{
}

impl<T> ProtocolTransactionId for T
where
    T: property::TransactionId + Debug,
    T: cbor_event::Deserialize + cbor_event::Serialize,
{
}
