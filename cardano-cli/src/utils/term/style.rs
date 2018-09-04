//! module that defines some pretty printing format styling
//! for given objects
//!
//! This module defines what style given cardano object should
//! adopt, this is in order to provide a consistent styling across
//! the application.

use console::{self};

use cardano::{hash, redeem, coin::{Coin}, block::{self, BlockDate, HeaderHash}, address::{self, ExtendedAddr}, config::ProtocolMagic, hdwallet};

pub use console::{StyledObject};

use super::super::super::transaction;
use super::super::super::wallet::{WalletName};

pub trait Style: Sized {
    fn style(self) -> StyledObject<Self>;
}

impl<'a> Style for &'a str {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
    }
}
impl<'a, T: Style> Style for &'a T {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
    }
}

impl Style for transaction::core::StagingId {
    fn style(self) -> StyledObject<Self> {
        console::style(self).white().bold().underlined()
    }
}
impl Style for Coin {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
            .green().bold()
    }
}
impl Style for BlockDate {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
            .white()
            .bold()
    }
}
impl Style for block::genesis::BodyProof {
    fn style(self) -> StyledObject<Self> {
        console::style(self).yellow()
    }
}
impl Style for block::types::EpochSlotId {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
            .white()
            .bold()
    }
}
impl Style for block::types::ChainDifficulty {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
    }
}
impl Style for hash::Blake2b256 {
    fn style(self) -> StyledObject<Self> {
        console::style(self).yellow()
    }
}
impl Style for HeaderHash {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
            .magenta()
    }
}
impl Style for ExtendedAddr {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
            .green()
            .italic()
    }
}
impl Style for ProtocolMagic {
    fn style(self) -> StyledObject<Self> {
        console::style(self)
    }
}
impl Style for hdwallet::XPub {
    fn style(self) -> StyledObject<Self> {
        console::style(self).green().italic()
    }
}
impl Style for hdwallet::XPrv {
    fn style(self) -> StyledObject<Self> {
        console::style(self).red()
    }
}
impl<A> Style for hdwallet::Signature<A> {
    fn style(self) -> StyledObject<Self> {
        console::style(self).cyan()
    }
}
impl Style for redeem::PublicKey {
    fn style(self) -> StyledObject<Self> {
        console::style(self).green().italic()
    }
}
impl Style for redeem::Signature {
    fn style(self) -> StyledObject<Self> {
        console::style(self).cyan()
    }
}
impl Style for address::StakeholderId {
    fn style(self) -> StyledObject<Self> {
        console::style(self).yellow().dim()
    }
}
impl Style for WalletName {
    fn style(self) -> StyledObject<Self> {
        console::style(self).white()
    }
}

macro_rules! impl_fmt {
    ($name:ident) => {
        impl Style for $name {
            fn style(self) -> StyledObject<Self> {
                console::style(self)
            }
        }
    }
}
impl_fmt!(String);
impl_fmt!(u8);
impl_fmt!(u16);
impl_fmt!(u32);
impl_fmt!(u64);
impl_fmt!(usize);
impl_fmt!(i8);
impl_fmt!(i16);
impl_fmt!(i32);
impl_fmt!(i64);
impl_fmt!(isize);

#[macro_export]
macro_rules! style {
    ($name:expr) => {
        Style::style($name)
    };
}
