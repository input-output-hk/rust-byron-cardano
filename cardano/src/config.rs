//! Blockchain network specific config (ProtocolMagic)
//!
//! there are some settings that need to be set in order to guarantee
//! operability with the appropriate network or different option.
//!

use cbor_event::{self, de::RawCbor, se::{Serializer}};
use std::fmt;

/// this is the protocol magic number
///
/// it is meant to be used on some places in order to guarantee
/// incompatibility between forks, test network and the main-net.
///
/// # Default
///
/// The default value is set to the mainnet
///
/// ```
/// use cardano::config::{ProtocolMagic};
///
/// assert_eq!(ProtocolMagic::default(), ProtocolMagic::new(0x2D964A09));
/// ```
///
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ProtocolMagic(u32);
impl ProtocolMagic {
    #[deprecated]
    pub fn new(val: u32) -> Self { ProtocolMagic(val) }
}
impl fmt::Display for ProtocolMagic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl ::std::ops::Deref for ProtocolMagic {
    type Target = u32;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl From<u32> for ProtocolMagic {
    fn from(v: u32) -> Self { ProtocolMagic(v) }
}
impl Default for ProtocolMagic {
    fn default() -> Self { ProtocolMagic::from(764824073) }
}
impl cbor_event::se::Serialize for ProtocolMagic {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_unsigned_integer(self.0 as u64)
    }
}
impl cbor_event::Deserialize for ProtocolMagic {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        let v = raw.unsigned_integer()? as u32;
        Ok(ProtocolMagic::from(v))
    }
}

/// Configuration for the wallet-crypto
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Config {
    pub protocol_magic: ProtocolMagic
}
impl Config {
    pub fn new(protocol_magic: ProtocolMagic) -> Self {
        Config {
            protocol_magic: protocol_magic
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Config::new(ProtocolMagic::default())
    }
}
