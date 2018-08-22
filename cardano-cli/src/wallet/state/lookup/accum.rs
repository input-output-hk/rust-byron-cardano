use cardano::hdwallet;
use cardano::address::ExtendedAddr;
use cardano::wallet::{rindex, bip44};

use super::{AddressLookup};
use super::super::{utxo::{UTxO}, log::{Log}};
use serde;

#[derive(Debug, Clone)]
pub struct Address();
impl ::std::fmt::Display for Address {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "Address")
    }
}
impl<'de> serde::Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::de::Deserializer<'de>
    {
        deserializer.deserialize_ignored_any(serde::de::IgnoredAny::default())?;
        Ok(Address())
    }
}

pub struct Accum();
impl Default for Accum { fn default() -> Self { Accum() } }

impl AddressLookup for Accum {
    type Error = ();
    type AddressInput = Address;
    type AddressOutput = Address;

    fn lookup(&mut self, utxo: UTxO<Self::AddressInput>) -> Result<Option<UTxO<Self::AddressOutput>>, Self::Error> {
        Ok(Some(utxo))
    }

    fn acknowledge(&mut self, _address: &Self::AddressOutput) -> Result<(), Self::Error> {
        Ok(())
    }
}
