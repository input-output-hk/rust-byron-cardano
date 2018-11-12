use super::types::{EpochId, EpochSlotId, SlotId};

use std::{
    cmp::{Ord, Ordering},
    error::Error,
    fmt,
    num::ParseIntError,
    str,
};

/// Block date, which is either an epoch id for a boundary block
/// or a slot id for a normal block.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "generic-serialization",
    derive(Serialize, Deserialize)
)]
pub enum BlockDate {
    Boundary(EpochId),
    Normal(EpochSlotId),
}
impl ::std::ops::Sub<BlockDate> for BlockDate {
    type Output = usize;
    fn sub(self, rhs: Self) -> Self::Output {
        self.slot_number() - rhs.slot_number()
    }
}

impl PartialOrd for BlockDate {
    fn partial_cmp(&self, other: &BlockDate) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BlockDate {
    fn cmp(&self, other: &BlockDate) -> Ordering {
        match self {
            BlockDate::Boundary(e1) => match other {
                    BlockDate::Boundary(e2) => e1.cmp(e2),
                    BlockDate::Normal(slot2) => e1.cmp(&slot2.epoch).then(Ordering::Less),
            },
            BlockDate::Normal(slot1) => match other {
                    BlockDate::Boundary(e2) => slot1.epoch.cmp(e2).then(Ordering::Greater),
                BlockDate::Normal(slot2) => slot1
                    .epoch
                    .cmp(&slot2.epoch)
                    .then(slot1.slotid.cmp(&slot2.slotid)),
            },
        }
    }
}

impl BlockDate {
    pub fn get_epochid(&self) -> EpochId {
        match self {
            &BlockDate::Boundary(e) => e,
            &BlockDate::Normal(ref s) => s.epoch,
        }
    }
    pub fn next(&self) -> Self {
        match self {
            &BlockDate::Boundary(e) => BlockDate::Normal(EpochSlotId {
                epoch: e,
                slotid: 0,
            }),
            &BlockDate::Normal(ref s) => BlockDate::Normal(s.next()), // TODO next should wrap after full epoch
        }
    }

    pub fn is_boundary(&self) -> bool {
        match self {
            BlockDate::Boundary(_) => true,
            _ => false,
        }
    }
    pub fn slot_number(&self) -> usize {
        match self {
            BlockDate::Boundary(eid) => (*eid as usize) * 21600, // TODO de-hardcode this value
            BlockDate::Normal(sid) => sid.slot_number(),
        }
    }
}

impl fmt::Display for BlockDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlockDate::Boundary(epoch) => write!(f, "{}.GENESIS", epoch),
            BlockDate::Normal(slotid) => write!(f, "{}.{}", slotid.epoch, slotid.slotid),
        }
    }
}
