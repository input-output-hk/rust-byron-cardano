use std::{str::{FromStr}, ops::{Deref, DerefMut}, fmt::{Display, Formatter}};
use cardano::util::try_from_slice::{TryFromSlice};
use serde;

/// Binary or Hexadecimal serde serialisation of objects based upton
/// FromStr, Display, AsRef<[u8]> and TryFromSlice
///
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Str<T>(pub T);
impl<T> From<T> for Str<T> {
    fn from(t: T) -> Self { Str(t) }
}
impl<T> Deref for Str<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl<T> DerefMut for Str<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
impl<T: AsRef<[u8]>> AsRef<[u8]> for Str<T> {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl<T: TryFromSlice> TryFromSlice for Str<T> {
    type Error = T::Error;
    fn try_from_slice(slice: &[u8]) -> Result<Self, Self::Error> {
        T::try_from_slice(slice).map(Str)
    }
}
impl<T: Display> Display for Str<T> {
    fn fmt(&self, f: &mut Formatter) -> ::std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl<T: FromStr> FromStr for Str<T> {
    type Err = T::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        T::from_str(s).map(Str)
    }
}

impl<T: Display> serde::ser::Serialize for Str<T> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}
impl<'de, T: FromStr> serde::Deserialize<'de> for Str<T>
    where <T as FromStr>::Err: Display
{
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        struct TVisitor<T>(::std::marker::PhantomData<T>);
        impl<'de, T: FromStr> serde::de::Visitor<'de> for TVisitor<T>
            where <T as FromStr>::Err: Display
        {
            type Value = T;
            fn expecting(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(fmt, "Expecting a Blake2b_256 hash (`Hash`)")
            }
            fn visit_str<'a, E>(self, v: &'a str) -> ::std::result::Result<Self::Value, E>
                where E: serde::de::Error
            {
                match Self::Value::from_str(&v) {
                    Err(err) => Err(E::custom(format!("{}", err))),
                    Ok(h) => Ok(h)
                }
            }
        }

        deserializer.deserialize_str(TVisitor(::std::marker::PhantomData))
    }
}


/// Binary or Hexadecimal serde serialisation of objects based upton
/// FromStr, Display, AsRef<[u8]> and TryFromSlice
///
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct BinOrStr<T>(pub T);
impl<T> From<T> for BinOrStr<T> {
    fn from(t: T) -> Self { BinOrStr(t) }
}
impl<T> Deref for BinOrStr<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { &self.0 }
}
impl<T> DerefMut for BinOrStr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}
impl<T: AsRef<[u8]>> AsRef<[u8]> for BinOrStr<T> {
    fn as_ref(&self) -> &[u8] { self.0.as_ref() }
}
impl<T: TryFromSlice> TryFromSlice for BinOrStr<T> {
    type Error = T::Error;
    fn try_from_slice(slice: &[u8]) -> Result<Self, Self::Error> {
        T::try_from_slice(slice).map(BinOrStr)
    }
}
impl<T: Display> Display for BinOrStr<T> {
    fn fmt(&self, f: &mut Formatter) -> ::std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
impl<T: FromStr> FromStr for BinOrStr<T> {
    type Err = T::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        T::from_str(s).map(BinOrStr)
    }
}

impl<T: AsRef<[u8]> + Display> serde::ser::Serialize for BinOrStr<T> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
        where S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&format!("{}", self))
        } else {
            serializer.serialize_bytes(&self.as_ref())
        }
    }
}
impl<'de, T: TryFromSlice + FromStr> serde::Deserialize<'de> for BinOrStr<T>
    where <T as FromStr>::Err: Display
        , <T as TryFromSlice>::Error: Display
{
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        struct TVisitor<T>(::std::marker::PhantomData<T>);
        impl<'de, T: TryFromSlice + FromStr> serde::de::Visitor<'de> for TVisitor<T>
            where <T as FromStr>::Err: Display
                , <T as TryFromSlice>::Error: Display
        {
            type Value = T;
            fn expecting(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
                write!(fmt, "Expecting a Blake2b_256 hash (`Hash`)")
            }
            fn visit_str<'a, E>(self, v: &'a str) -> ::std::result::Result<Self::Value, E>
                where E: serde::de::Error
            {
                match Self::Value::from_str(&v) {
                    Err(err) => Err(E::custom(format!("{}", err))),
                    Ok(h) => Ok(h)
                }
            }
            fn visit_bytes<'a, E>(self, v: &'a [u8]) -> ::std::result::Result<Self::Value, E>
                where E: serde::de::Error
            {
                match Self::Value::try_from_slice(v) {
                    Err(err) => panic!("unexpected error: {}", err),
                    Ok(h) => Ok(h)
                }
            }
        }

        if deserializer.is_human_readable() {
            deserializer.deserialize_str(TVisitor(::std::marker::PhantomData))
        } else {
            deserializer.deserialize_bytes(TVisitor(::std::marker::PhantomData))
        }
    }
}
