use cardano::{wallet::{bip44, rindex}, address::{ExtendedAddr}};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Address {
    Bip44(bip44::Addressing),
    RIndex(rindex::Addressing),
    Unknown(ExtendedAddr)
}
impl ::std::fmt::Display for Address {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match self {
            Address::Bip44(address) => write!(f, "{}", address),
            Address::RIndex(address) => write!(f, "{}", address),
            Address::Unknown(address) => write!(f, "{}", address),
        }
    }
}
impl From<bip44::Addressing> for Address {
    fn from(address: bip44::Addressing) -> Self { Address::Bip44(address) }
}
impl From<rindex::Addressing> for Address {
    fn from(address: rindex::Addressing) -> Self { Address::RIndex(address) }
}
impl From<ExtendedAddr> for Address {
    fn from(address: ExtendedAddr) -> Self { Address::Unknown(address) }
}
impl<'a> From<&'a bip44::Addressing> for Address {
    fn from(address: &'a bip44::Addressing) -> Self { Address::Bip44(address.clone()) }
}
impl<'a> From<&'a rindex::Addressing> for Address {
    fn from(address: &'a rindex::Addressing) -> Self { Address::RIndex(address.clone()) }
}
impl<'a> From<&'a ExtendedAddr> for Address {
    fn from(address: &'a ExtendedAddr) -> Self { Address::Unknown(address.clone()) }
}
