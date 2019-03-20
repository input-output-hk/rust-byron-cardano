use super::types::{EpochId, EpochSlotId, SlotId};
use chain_core::property;

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
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum BlockDate {
    Boundary(EpochId),
    Normal(EpochSlotId),
}

impl property::BlockDate for BlockDate {
    fn from_epoch_slot_id(epoch: u32, slot_id: u32) -> Self {
        BlockDate::Normal(EpochSlotId {
            epoch: epoch as u64,
            slotid: slot_id as u16,
        })
    }
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
    pub fn slotid(&self) -> Option<SlotId> {
        match self {
            &BlockDate::Boundary(_) => None,
            &BlockDate::Normal(ref s) => Some(s.slotid),
        }
    }
    pub fn epoch_and_slot(&self) -> (EpochId, Option<SlotId>) {
        (self.get_epochid(), self.slotid())
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
impl From<EpochSlotId> for BlockDate {
    fn from(esi: EpochSlotId) -> Self {
        BlockDate::Normal(esi)
    }
}
impl From<EpochId> for BlockDate {
    fn from(ei: EpochId) -> Self {
        BlockDate::Boundary(ei)
    }
}

impl str::FromStr for BlockDate {
    type Err = BlockDateParseError;

    fn from_str(s: &str) -> Result<BlockDate, BlockDateParseError> {
        use self::ParseErrorKind::*;

        let (ep, opt_sp) = match s.find('.') {
            None => (s, None),
            Some(pos) => (&s[..pos], Some(&s[(pos + 1)..])),
        };
        let epoch = str::parse::<EpochId>(ep).map_err(|e| BlockDateParseError(BadEpochId(e)))?;
        match opt_sp {
            None => Ok(BlockDate::Boundary(epoch)),
            Some(sp) => {
                if sp == "GENESIS" {
                    return Ok(BlockDate::Boundary(epoch));
                }
                let slotid =
                    str::parse::<SlotId>(sp).map_err(|e| BlockDateParseError(BadSlotId(e)))?;
                Ok(BlockDate::Normal(EpochSlotId { epoch, slotid }))
            }
        }
    }
}

#[derive(Debug)]
pub struct BlockDateParseError(ParseErrorKind);

#[derive(Debug)]
enum ParseErrorKind {
    BadEpochId(ParseIntError),
    BadSlotId(ParseIntError),
}

const EXPECT_FORMAT_MESSAGE: &'static str = "expected block date format EPOCH[.SLOT]";

impl fmt::Display for BlockDateParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::ParseErrorKind::*;
        match self.0 {
            BadEpochId(_) => write!(f, "invalid epoch ID, {}", EXPECT_FORMAT_MESSAGE),
            BadSlotId(_) => write!(f, "invalid slot ID, {}", EXPECT_FORMAT_MESSAGE),
        }
    }
}

impl Error for BlockDateParseError {
    fn cause(&self) -> Option<&dyn Error> {
        use self::ParseErrorKind::*;
        match self.0 {
            BadEpochId(ref e) => Some(e),
            BadSlotId(ref e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BlockDate;
    use block::EpochSlotId;
    use std::error::Error;

    #[test]
    fn parse_bare_epoch() {
        let date = "42".parse::<BlockDate>().unwrap();
        assert_eq!(date, BlockDate::Boundary(42));
    }

    #[test]
    fn parse_epoch_slot_id() {
        let date = "42.12".parse::<BlockDate>().unwrap();
        assert_eq!(
            date,
            BlockDate::Normal(EpochSlotId {
                epoch: 42,
                slotid: 12
            })
        );
    }

    #[test]
    fn parse_epoch_genesis() {
        let date = "42.GENESIS".parse::<BlockDate>().unwrap();
        assert_eq!(date, BlockDate::Boundary(42));
    }

    #[test]
    fn parse_bad_epoch() {
        let err = "".parse::<BlockDate>().unwrap_err();
        println!("{}: {}", err, err.cause().unwrap());
    }

    #[test]
    fn parse_bad_slotid() {
        let err = "42.INVALID".parse::<BlockDate>().unwrap_err();
        println!("{}: {}", err, err.cause().unwrap());
    }
}
