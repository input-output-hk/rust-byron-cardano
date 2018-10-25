use std::{ops::{Deref}};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(u64);
impl NodeId {
    pub fn next(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}
impl From<u64> for NodeId { fn from(v: u64) -> Self { NodeId(v) } }
impl Default for NodeId { fn default() -> Self { NodeId(0) } }
impl Deref for NodeId {
    type Target = u64;
    fn deref(&self) -> &Self::Target { &self.0 }
}
