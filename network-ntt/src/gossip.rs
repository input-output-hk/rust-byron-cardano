//! Compatibility stubs for network-core gossip traits

use chain_core::property;
use network_core::gossip as core_gossip;

use std::io;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(protocol::protocol::NodeId);

impl property::Serialize for NodeId {
    type Error = io::Error;

    fn serialize<W: std::io::Write>(&self, _writer: W) -> Result<(), Self::Error> {
        unimplemented!()
    }
}

impl property::Deserialize for NodeId {
    type Error = io::Error;

    fn deserialize<R: std::io::BufRead>(_reader: R) -> Result<Self, Self::Error> {
        unimplemented!()
    }
}

impl core_gossip::NodeId for NodeId {}
