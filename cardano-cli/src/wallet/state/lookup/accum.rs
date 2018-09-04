use super::{AddressLookup, Address};
use super::super::{utxo::{UTxO}};
use cardano::address::ExtendedAddr;


pub struct Accum();
impl Default for Accum { fn default() -> Self { Accum() } }

impl AddressLookup for Accum {
    type Error = ();

    fn lookup(&mut self, utxo: UTxO<ExtendedAddr>) -> Result<Option<UTxO<Address>>, Self::Error> {
        Ok(Some(utxo.map(|a| a.into())))
    }

    fn acknowledge<A: Into<Address>>(&mut self, _: A) -> Result<(), Self::Error> {
        Ok(())
    }
}
