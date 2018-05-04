//! there are some settings that need to be set in order to guarantee
//! operability with the appropriate network or different option.
//!

use cbor;
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
/// use wallet_crypto::config::{ProtocolMagic};
///
/// assert_eq!(ProtocolMagic::default(), ProtocolMagic::new(0x2D964A09));
/// ```
///
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct ProtocolMagic(u32);
impl ProtocolMagic {
    pub fn new(val: u32) -> Self { ProtocolMagic(val) }
}
impl fmt::Display for ProtocolMagic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl cbor::CborValue for ProtocolMagic {
    fn encode(&self) -> cbor::Value { cbor::CborValue::encode(&self.0) }
    fn decode(value: cbor::Value) -> cbor::Result<Self> {
        let v : u32 = cbor::CborValue::decode(value)?;
        Ok(ProtocolMagic::new(v))
    }
}
impl Default for ProtocolMagic {
    fn default() -> Self { ProtocolMagic::new(764824073) }
}

/// Configuration for the wallet-crypto
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
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
