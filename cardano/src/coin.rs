//! Cardano's Lovelace value
//!
//! This represents the type value and has some properties associated
//! such as a min bound of 0 and a max bound of `MAX_COIN`.
//!

use cbor_event::{self, de::RawCbor, se::{Serializer}};
use std::{ops, fmt, result};

/// maximum value of a Lovelace.
pub const MAX_COIN: u64 = 45_000_000_000__000_000;

/// error type relating to `Coin` operations
///
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Error {
    /// means that the given value was out of bound
    ///
    /// Max bound being: `MAX_COIN`.
    OutOfBound(u64),

    ParseIntError,

    Negative
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::OutOfBound(ref v) => write!(f, "Coin of value {} is out of bound. Max coin value: {}.", v, MAX_COIN),
            &Error::ParseIntError     => write!(f, "Cannot parse a valid integer"),
            &Error::Negative          => write!(f, "Coin cannot hold a negative value"),
        }
    }
}

pub type Result<T> = result::Result<T, Error>;

// TODO: add custom implementation of `serde::de::Deserialize` so we can check the
// upper bound of the `Coin`.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Coin(u64);
impl Coin {
    /// create a coin of value `0`.
    ///
    /// # Example
    ///
    /// ```
    /// use cardano::coin::{Coin};
    ///
    /// println!("{}", Coin::zero());
    /// ```
    pub fn zero() -> Self { Coin(0) }

    /// create of unitary coin (a coin of value `1`)
    ///
    /// # Example
    ///
    /// ```
    /// use cardano::coin::{Coin};
    ///
    /// println!("{}", Coin::unit());
    /// ```
    pub fn unit() -> Self { Coin(1) }

    /// create a coin of the given value
    ///
    /// # Example
    ///
    /// ```
    /// use cardano::coin::{Coin};
    ///
    /// let coin = Coin::new(42);
    /// let invalid = Coin::new(45000000000000001);
    ///
    /// assert!(coin.is_ok());
    /// assert!(invalid.is_err());
    /// ```
    pub fn new(v: u64) -> Result<Self> {
        if v <= MAX_COIN { Ok(Coin(v)) } else { Err(Error::OutOfBound(v)) }
    }
}
impl ::std::ops::Deref for Coin {
    type Target = u64;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl fmt::Display for Coin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{:06}", self.0 / 1000000, self.0 % 1000000)
    }
}
impl ::std::str::FromStr for Coin {
    type Err = Error;
    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        let v : u64 = match s.parse() {
            Err(_) => return Err(Error::ParseIntError),
            Ok(v) => v
        };
        Coin::new(v)
    }
}
impl cbor_event::se::Serialize for Coin {
    fn serialize<W: ::std::io::Write>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>> {
        serializer.write_unsigned_integer(self.0)
    }
}
impl cbor_event::de::Deserialize for Coin {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> cbor_event::Result<Self> {
        Coin::new(raw.unsigned_integer()?).map_err(|err| {
            cbor_event::Error::CustomError(format!("{}", err))
        })
    }
}
impl ops::Add for Coin {
    type Output = Result<Coin>;
    fn add(self, other: Coin) -> Self::Output {
        Coin::new(self.0 + other.0)
    }
}
impl<'a> ops::Add<&'a Coin> for Coin {
    type Output = Result<Coin>;
    fn add(self, other: &'a Coin) -> Self::Output {
        Coin::new(self.0 + other.0)
    }
}
impl ops::Sub for Coin {
    type Output = Result<Coin>;
    fn sub(self, other: Coin) -> Self::Output {
        if other.0 > self.0 {
            Err(Error::Negative)
        } else {
            Ok(Coin(self.0 - other.0))
        }
    }
}
impl<'a> ops::Sub<&'a Coin> for Coin {
    type Output = Result<Coin>;
    fn sub(self, other: &'a Coin) -> Self::Output {
        if other.0 > self.0 {
            Err(Error::Negative)
        } else {
            Ok(Coin(self.0 - other.0))
        }
    }
}
// this instance is necessary to chain the substraction operations
//
// i.e. `coin1 - coin2 - coin3`
impl ops::Sub<Coin> for Result<Coin> {
    type Output = Result<Coin>;
    fn sub(self, other: Coin) -> Self::Output {
        if other.0 > self?.0 {
            Err(Error::Negative)
        } else {
            Ok(Coin(self?.0 - other.0))
        }
    }
}

impl From<Coin> for u64 {
    fn from(c: Coin) -> u64 { c.0 }
}

impl From<u32> for Coin {
    fn from(c: u32) -> Coin { Coin(c as u64) }
}

pub fn sum_coins<I>(coin_iter: I) -> Result<Coin>
    where I: Iterator<Item = Coin>
{
    coin_iter.fold(Coin::new(0), |acc, ref c| acc.and_then(|v| v + *c))
}
