use quickcheck::{Arbitrary, Gen};
use std::ops::Deref;

use super::super::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Wrapper<A>(A);
impl<A> Wrapper<A> {
    pub fn unwrap(self) -> A {
        self.0
    }
}
impl<A> Deref for Wrapper<A> {
    type Target = A;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<A> AsRef<A> for Wrapper<A> {
    fn as_ref(&self) -> &A {
        &self.0
    }
}
impl<A> From<A> for Wrapper<A> {
    fn from(a: A) -> Self {
        Wrapper(a)
    }
}

impl Arbitrary for Wrapper<config::ProtocolMagic> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        Wrapper(config::ProtocolMagic::from(<u32 as Arbitrary>::arbitrary(
            g,
        )))
    }
}

impl Arbitrary for Wrapper<config::NetworkMagic> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        if Arbitrary::arbitrary(g) {
            Wrapper(config::NetworkMagic::NoMagic)
        } else {
            Wrapper(config::NetworkMagic::Magic(Arbitrary::arbitrary(g)))
        }
    }
}

impl Arbitrary for Wrapper<coin::Coin> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let value = u64::arbitrary(g) % coin::MAX_COIN;
        coin::Coin::new(value).unwrap().into()
    }
}

impl Arbitrary for Wrapper<hdwallet::Seed> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let vec: Vec<u8> = ::std::iter::repeat_with(|| Arbitrary::arbitrary(g))
            .take(hdwallet::SEED_SIZE)
            .collect();
        Wrapper(hdwallet::Seed::from_slice(&vec).unwrap())
    }
}

impl Arbitrary for Wrapper<hdwallet::XPrv> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let seed: Wrapper<_> = Arbitrary::arbitrary(g);
        Wrapper(hdwallet::XPrv::generate_from_seed(&seed))
    }
}

impl Arbitrary for Wrapper<(hdwallet::XPrv, address::ExtendedAddr)> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let xprv: Wrapper<hdwallet::XPrv> = Arbitrary::arbitrary(g);
        let network_magic: Wrapper<config::NetworkMagic> = Arbitrary::arbitrary(g);
        let attributes = address::Attributes {
            derivation_path: None,
            stake_distribution: address::StakeDistribution::BootstrapEraDistr,
            network_magic: *network_magic,
        };
        let addr_type = address::AddrType::ATPubKey;
        let addr = address::HashedSpendingData::new(
            addr_type,
            &address::SpendingData::PubKeyASD(xprv.public()),
            &attributes,
        );
        let address = address::ExtendedAddr {
            addr: addr,
            attributes: attributes,
            addr_type: addr_type,
        };
        Wrapper((xprv.unwrap(), address))
    }
}

impl Arbitrary for Wrapper<hash::Blake2b256> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        use super::try_from_slice::TryFromSlice;

        let vec: Vec<u8> = ::std::iter::repeat_with(|| Arbitrary::arbitrary(g))
            .take(hash::Blake2b256::HASH_SIZE)
            .collect();

        let hash = hash::Blake2b256::try_from_slice(&vec).unwrap();
        Wrapper(hash)
    }
}

impl Arbitrary for Wrapper<tx::TxoPointer> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let txid: Wrapper<tx::TxId> = Arbitrary::arbitrary(g);
        Wrapper(tx::TxoPointer {
            id: txid.unwrap(),
            index: Arbitrary::arbitrary(g),
        })
    }
}

impl Arbitrary for Wrapper<(hdwallet::XPrv, tx::TxOut)> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let address: Wrapper<(hdwallet::XPrv, address::ExtendedAddr)> = Arbitrary::arbitrary(g);
        let value: Wrapper<coin::Coin> = Arbitrary::arbitrary(g);
        let (xprv, address) = address.unwrap();
        Wrapper((
            xprv,
            tx::TxOut {
                address: address,
                value: value.unwrap(),
            },
        ))
    }
}

impl<A: Arbitrary> Arbitrary for Wrapper<(hdwallet::XPrv, txutils::Input<A>)> {
    fn arbitrary<G: Gen>(g: &mut G) -> Self {
        let value: Wrapper<(hdwallet::XPrv, tx::TxOut)> = Arbitrary::arbitrary(g);
        let ptr: Wrapper<tx::TxoPointer> = Arbitrary::arbitrary(g);
        let addressing: A = Arbitrary::arbitrary(g);
        let (xprv, txout) = value.unwrap();
        Wrapper((
            xprv,
            txutils::Input {
                ptr: ptr.unwrap(),
                value: txout,
                addressing: addressing,
            },
        ))
    }
}
