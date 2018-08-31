//! CBOR deserialisation tooling

use std::{self, fmt, ops::{Deref}, collections::BTreeMap};
use error::Error;
use result::Result;
use types::{Type, Special, Bytes};
use len::Len;

pub trait Deserialize : Sized {
    /// method to implement to deserialise an object from the given
    /// `RawCbor`.
    fn deserialize<'a>(&mut RawCbor<'a>) -> Result<Self>;
}

impl Deserialize for u8 {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        let n = raw.unsigned_integer()?;
        if n > std::u8::MAX as u64 {
            Err(Error::ExpectedU8)
        } else {
            Ok(n as Self)
        }
    }
}

impl Deserialize for u16 {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        let n = raw.unsigned_integer()?;
        if n > std::u16::MAX as u64 {
            Err(Error::ExpectedU16)
        } else {
            Ok(n as Self)
        }
    }
}

impl Deserialize for u32 {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        let n = raw.unsigned_integer()?;
        if n > std::u32::MAX as u64 {
            Err(Error::ExpectedU32)
        } else {
            Ok(n as Self)
        }
    }
}

impl Deserialize for u64 {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        raw.unsigned_integer()
    }
}

impl Deserialize for bool {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        raw.bool()
    }
}

impl Deserialize for String {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<String> {
        raw.text()
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        let len = raw.array()?;
        let mut vec = Vec::new();
        match len {
            Len::Indefinite => {
                while {
                    let t = raw.cbor_type()?;
                    if t == Type::Special {
                        let special = raw.special()?;
                        assert_eq!(special, Special::Break);
                        false
                    } else {
                        vec.push(Deserialize::deserialize(raw)?);
                        true
                    }
                } {};
            },
            Len::Len(len) => {
                for _ in 0..len {
                    vec.push(Deserialize::deserialize(raw)?);
                }
            }
        }
        Ok(vec)
    }
}
impl<K: Deserialize+Ord, V: Deserialize> Deserialize for BTreeMap<K,V> {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        let len = raw.map()?;
        let mut vec = BTreeMap::new();
        match len {
            Len::Indefinite => {
                while {
                    let t = raw.cbor_type()?;
                    if t == Type::Special {
                        let special = raw.special()?;
                        assert_eq!(special, Special::Break);
                        false
                    } else {
                        let k = Deserialize::deserialize(raw)?;
                        let v = Deserialize::deserialize(raw)?;
                        vec.insert(k, v);
                        true
                    }
                } {};
            },
            Len::Len(len) => {
                for _ in 0..len {
                    let k = Deserialize::deserialize(raw)?;
                    let v = Deserialize::deserialize(raw)?;
                    vec.insert(k, v);
                }
            }
        }
        Ok(vec)
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        match raw.array()? {
            Len::Len(0) => Ok(None),
            Len::Len(1) => Ok(Some(raw.deserialize()?)),
            len => Err(Error::CustomError(format!("Invalid Option<T>: received array of {:?} elements", len)))
        }
    }
}

/// [`Deserialize`]: ./trait.Deserialize.html
/// [`Error`]: ../enum.Error.html
/// [`Type`]: ../enum.Type.html
/// [`Bytes`]: ../struct.Bytes.html
/// [`cbor_type`]: #method.cbor_type
/// [`cbor_len`]: #method.cbor_len
/// [`Len`]: ../enum.Len.html
///
/// `RawCbor` represents a chunk of bytes believed to be cbor object.
/// The validity of the cbor bytes is known only when trying
/// to get meaningful cbor objects from it.
///
/// # Examples
///
/// If you already know the CBOR Primary [`Type`] you are expecting, you
/// can efficiently use the appropriate commands:
///
/// ```
/// use cbor_event::de::*;
///
/// let vec = vec![0x18, 0x40];
/// let mut raw = RawCbor::from(&vec);
///
/// assert!(raw.unsigned_integer().is_ok());
/// ```
///
/// ```
/// use cbor_event::de::*;
///
/// let vec = vec![0x18, 0x40];
/// let mut raw = RawCbor::from(&vec);
///
/// assert!(raw.array().is_err());
/// ```
///
/// If you don't know the [`Type`] and are only analyzing the structure, you
/// can use [`cbor_type`] to get the type of the next object to parse.
///
/// # Ownership
///
/// [`RawCbor`] does not take ownership of the underlying slice. It takes
/// a reference and binds its lifetime to the owner of that slice.
///
/// ```
/// use cbor_event::de::*;
///
/// let vec = vec![0,1,2,3];
/// let raw = RawCbor::from(&vec);
/// ```
///
/// ```
/// use cbor_event::de::*;
///
/// // here the integer takes ownership of its own data as this is a
/// // simple enough type
/// let integer = {
///     let vec = vec![0x18, 0x40];
///     let mut raw = RawCbor::from(&vec);
///     raw.unsigned_integer().unwrap()
/// };
///
/// assert_eq!(64, integer);
///
/// ```
///
/// But here the returned [`Bytes`] object's lifetime is bound to the lifetime
/// of the `RawCbor` object.
///
/// ```compile_fail
/// use cbor_event::de::*;
///
/// // here this won't compile because the bytes' lifetime is bound
/// // to the parent RawCbor (variable `raw`, which is bound to `vec`).
/// let bytes = {
///     let vec = vec![0x43, 0x01, 0x02, 0x03];
///     let mut raw = RawCbor::from(&vec);
///     raw.bytes().unwrap()
/// };
/// ```
///
/// This is in order to allow fast skipping of cbor value to reach to desired
/// specific data.
///
/// # Error
///
/// When deserialising from `RawCbor` it is possible to see the following
/// [`Error`]s:
///
/// - `Error::NotEnough(current_size, needed_size)`: meaning we are expecting
///   a more bytes to parse the CBOR properly;
/// - `Error::Expected(expected_type, current_type)`: the current cbor primary
///   [`Type`] is different from the expected [`Type`];
/// - `Error::UnknownLenType(byte)`: the CBOR is serialized in an unknown
///   or unsupported format;
/// - `Error::IndefiniteLenUnsupported(t)`: the Indefinite length is not
///   supported for the given [`Type`] `t`;
/// - `Error::IoError(io_error)`: error due relating to buffer management;
///
/// # Panic
///
/// There is no explicit `panic!` in this code, except a few `unreachable!`.
///
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct RawCbor<'a>(&'a [u8]);
impl<'a> fmt::Display for RawCbor<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for i in self.iter() {
            write!(f, "{:02x}", i)?;
        }
        Ok(())
    }
}
impl<'a> RawCbor<'a> {
    #[inline]
    fn get(&self, index: usize) -> Result<u8> {
        match self.0.get(index) {
            None => Err(Error::NotEnough(self.len(), index)),
            Some(b) => Ok(*b)
        }
    }
    #[inline]
    fn u8(&self, index: usize) -> Result<u64> {
        let b = self.get(index)?;
        Ok(b as u64)
    }
    #[inline]
    fn u16(&self, index: usize) -> Result<u64> {
        let b1 = self.u8(index)?;
        let b2 = self.u8(index + 1)?;
        Ok(b1 << 8 | b2)
    }
    #[inline]
    fn u32(&self, index: usize) -> Result<u64> {
        let b1 = self.u8(index)?;
        let b2 = self.u8(index + 1)?;
        let b3 = self.u8(index + 2)?;
        let b4 = self.u8(index + 3)?;
        Ok(b1 << 24 | b2 << 16 | b3 << 8 | b4)
    }
    #[inline]
    fn u64(&self, index: usize) -> Result<u64> {
        let b1 = self.u8(index)?;
        let b2 = self.u8(index + 1)?;
        let b3 = self.u8(index + 2)?;
        let b4 = self.u8(index + 3)?;
        let b5 = self.u8(index + 4)?;
        let b6 = self.u8(index + 5)?;
        let b7 = self.u8(index + 6)?;
        let b8 = self.u8(index + 7)?;
        Ok(b1 << 56 | b2 << 48 | b3 << 40 | b4 << 32 | b5 << 24 | b6 << 16 | b7 << 8 | b8)
    }

    /// function to extract the type of the given `RawCbor`.
    ///
    /// This function does not consume the underlying buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use cbor_event::{de::*, Type};
    ///
    /// let vec = vec![0x18, 0x40];
    /// let mut raw = RawCbor::from(&vec);
    /// let cbor_type = raw.cbor_type().unwrap();
    ///
    /// assert!(cbor_type == Type::UnsignedInteger);
    /// ```
    #[inline]
    pub fn cbor_type(&self) -> Result<Type> {
        Ok(Type::from(self.get(0)?))
    }
    #[inline]
    fn cbor_expect_type(&self, t: Type) -> Result<()> {
        let t_ = self.cbor_type()?;
        if t_ != t {
            Err(Error::Expected(t, t_))
        } else {
            Ok(())
        }
    }

    /// function to extract the get the length parameter of
    /// the given cbor object. The returned tuple contains
    ///
    /// [`Type`]: ../enum.Type.html
    /// [`Len`]: ../enum.Len.html
    ///
    /// * the [`Len`];
    /// * the size of the encoded length (the number of bytes the data was encoded in).
    ///   `0` means the length is `< 24` and was encoded along the `Type`.
    ///
    /// If you are expecting a `Type` `UnsignedInteger` or `NegativeInteger` the meaning of
    /// the length is slightly different:
    ///
    /// * `Len::Indefinite` is an error;
    /// * `Len::Len(len)` is the read value of the integer.
    ///
    /// This function does not consume the underlying buffer.
    ///
    /// # Examples
    ///
    /// ```
    /// use cbor_event::{de::*, Len};
    ///
    /// let vec = vec![0x83, 0x01, 0x02, 0x03];
    /// let mut raw = RawCbor::from(&vec);
    /// let (len, len_sz) = raw.cbor_len().unwrap();
    ///
    /// assert_eq!(len, Len::Len(3));
    /// assert_eq!(len_sz, 0);
    /// ```
    ///
    #[inline]
    pub fn cbor_len(&self) -> Result<(Len, usize)> {
        let b : u8 = self.get(0)? & 0b0001_1111;
        match b {
            0x00..=0x17 => { Ok((Len::Len(b as u64), 0)) },
            0x18        => { self.u8(1).map(|v| (Len::Len(v), 1)) },
            0x19        => { self.u16(1).map(|v| (Len::Len(v), 2)) },
            0x1a        => { self.u32(1).map(|v| (Len::Len(v), 4)) },
            0x1b        => { self.u64(1).map(|v| (Len::Len(v), 8)) },
            0x1c..=0x1e => Err(Error::UnknownLenType(b)),
            0x1f        => Ok((Len::Indefinite, 0)),

            // since the value `b` has been masked to only consider the first 5 lowest bits
            // all value above 0x1f are unreachable.
            _ => unreachable!()
        }
    }

    /// consume the given `len` from the underlying buffer. Skipped bytes are
    /// then lost, they cannot be retrieved for future references.
    #[inline]
    pub fn advance(&mut self, len: usize) -> Result<()> {
        if self.0.len() < len {
            Err(Error::NotEnough(self.len(), len))
        } else {
            Ok(self.0 = &self.0[len..])
        }
    }

    /// Read an `UnsignedInteger` from the `RawCbor`
    ///
    /// The function fails if the type of the given RawCbor is not `Type::UnsignedInteger`.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::de::{*};
    ///
    /// let vec = vec![0x18, 0x40];
    /// let mut raw = RawCbor::from(&vec);
    ///
    /// let integer = raw.unsigned_integer().unwrap();
    ///
    /// assert_eq!(integer, 64);
    /// ```
    ///
    /// ```should_panic
    /// use cbor_event::de::{*};
    ///
    /// let vec = vec![0x83, 0x01, 0x02, 0x03];
    /// let mut raw = RawCbor::from(&vec);
    ///
    /// // the following line will panic:
    /// let integer = raw.unsigned_integer().unwrap();
    /// ```
    pub fn unsigned_integer(&mut self) -> Result<u64> {
        self.cbor_expect_type(Type::UnsignedInteger)?;
        let (len, len_sz) = self.cbor_len()?;
        match len {
            Len::Indefinite => Err(Error::IndefiniteLenNotSupported(Type::UnsignedInteger)),
            Len::Len(v) => {
                self.advance(1 + len_sz)?;
                Ok(v)
            }
        }
    }

    /// Read a `NegativeInteger` from the `RawCbor`
    ///
    /// The function fails if the type of the given RawCbor is not `Type::NegativeInteger`.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::de::{*};
    ///
    /// let vec = vec![0x38, 0x29];
    /// let mut raw = RawCbor::from(&vec);
    ///
    /// let integer = raw.negative_integer().unwrap();
    ///
    /// assert_eq!(integer, -42);
    /// ```
    pub fn negative_integer(&mut self) -> Result<i64> {
        self.cbor_expect_type(Type::NegativeInteger)?;
        let (len, len_sz) = self.cbor_len()?;
        match len {
            Len::Indefinite => Err(Error::IndefiniteLenNotSupported(Type::NegativeInteger)),
            Len::Len(v)     => {
                self.advance(1 + len_sz)?;
                Ok(- (v as i64) - 1)
            }
        }
    }

    /// Read a Bytes from the RawCbor
    ///
    /// The function fails if the type of the given RawCbor is not `Type::Bytes`.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::de::{*};
    ///
    /// let vec = vec![0x52, 0x73, 0x6F, 0x6D, 0x65, 0x20, 0x72, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x20, 0x73, 0x74, 0x72, 0x69, 0x6E, 0x67];
    /// let mut raw = RawCbor::from(&vec);
    ///
    /// let bytes = raw.bytes().unwrap();
    /// ```
    pub fn bytes(&mut self) -> Result<Bytes<'a>> {
        self.cbor_expect_type(Type::Bytes)?;
        let (len, len_sz) = self.cbor_len()?;
        match len {
            Len::Indefinite => Err(Error::IndefiniteLenNotSupported(Type::Bytes)),
            Len::Len(len) => {
                let start = 1 + len_sz;
                let end   = start + len as usize;
                let bytes = Bytes::from(&self.0[start..end as usize]);
                self.advance(end)?;
                Ok(bytes)
            }
        }
    }

    /// Read a Text from the RawCbor
    ///
    /// The function fails if the type of the given RawCbor is not `Type::Text`.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::de::{*};
    ///
    /// let vec = vec![0x64, 0x74, 0x65, 0x78, 0x74];
    /// let mut raw = RawCbor::from(&vec);
    ///
    /// let text = raw.text().unwrap();
    ///
    /// assert!(&*text == "text");
    /// ```
    pub fn text(&mut self) -> Result<String> {
        self.cbor_expect_type(Type::Text)?;
        let (len, len_sz) = self.cbor_len()?;
        match len {
            Len::Indefinite => Err(Error::IndefiniteLenNotSupported(Type::Text)),
            Len::Len(len) => {
                let start = 1 + len_sz;
                let end   = start + len as usize;
                let bytes = &self.0[start..end as usize];
                let text = String::from_utf8(Vec::from(bytes))?;
                self.advance(end)?;
                Ok(text)
            }
        }
    }

    /// cbor array of cbor objects
    ///
    /// The function fails if the type of the given RawCbor is not `Type::Array`.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::{de::{*}, Len};
    ///
    /// let vec = vec![0x86, 0,1,2,3,4,5];
    /// let mut raw = RawCbor::from(&vec);
    ///
    /// let len = raw.array().unwrap();
    ///
    /// assert_eq!(len, Len::Len(6));
    /// assert_eq!(raw.as_ref(), &[0,1,2,3,4,5][..]);
    /// ```
    ///
    pub fn array(&mut self) -> Result<Len> {
        self.cbor_expect_type(Type::Array)?;
        let (len, sz) = self.cbor_len()?;
        self.advance(1+sz)?;
        Ok(len)
    }

    /// Expect an array of a specified length. Must be a definite-length array.
    pub fn tuple(&mut self, expected_len: u64, error_location: &'static str) -> Result<()> {
        let actual_len = self.array()?;
        match actual_len {
            Len::Len(len) if expected_len == len => Ok(()),
            _ => Err(Error::WrongLen(expected_len, actual_len, error_location)),
        }
    }

    /// cbor map
    ///
    /// The function fails if the type of the given RawCbor is not `Type::Map`.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::{de::{*}, Len};
    ///
    /// let vec = vec![0xA2, 0x00, 0x64, 0x74, 0x65, 0x78, 0x74, 0x01, 0x18, 0x2A];
    /// let mut raw = RawCbor::from(&vec);
    ///
    /// let len = raw.map().unwrap();
    ///
    /// assert_eq!(len, Len::Len(2));
    /// ```
    ///
    pub fn map(&mut self) -> Result<Len> {
        self.cbor_expect_type(Type::Map)?;
        let (len, sz) = self.cbor_len()?;
        self.advance(1+sz)?;
        Ok(len)
    }

    /// Cbor Tag
    ///
    /// The function fails if the type of the given RawCbor is not `Type::Tag`.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::{de::{*}, Len};
    ///
    /// let vec = vec![0xD8, 0x18, 0x64, 0x74, 0x65, 0x78, 0x74];
    /// let mut raw = RawCbor::from(&vec);
    ///
    /// let tag = raw.tag().unwrap();
    ///
    /// assert_eq!(24, tag);
    /// assert_eq!("text", &*raw.text().unwrap());
    /// ```
    ///
    pub fn tag(&mut self) -> Result<u64> {
        self.cbor_expect_type(Type::Tag)?;
        match self.cbor_len()? {
            (Len::Indefinite, _) => Err(Error::IndefiniteLenNotSupported(Type::Tag)),
            (Len::Len(len), sz) => {
                self.advance(1+sz)?;
                Ok(len)
            }
        }
    }

    pub fn set_tag(&mut self) -> Result<()> {
        let tag = self.tag()?;
        if tag != 258 {
            return Err(Error::ExpectedSetTag);
        }
        Ok(())
    }

    pub fn special(&mut self) -> Result<Special> {
        self.cbor_expect_type(Type::Special)?;
        let b = self.get(0)? & 0b0001_1111;
        match b {
            0x00..=0x13 => { self.advance(1)?; Ok(Special::Unassigned(b)) },
            0x14        => { self.advance(1)?; Ok(Special::Bool(false)) },
            0x15        => { self.advance(1)?; Ok(Special::Bool(true)) },
            0x16        => { self.advance(1)?; Ok(Special::Null) },
            0x17        => { self.advance(1)?; Ok(Special::Undefined) },
            0x18        => { let b = self.u8(1)?;  self.advance(2)?; Ok(Special::Unassigned(b as u8)) },
            0x19        => { let f = self.u16(1)?; self.advance(3)?; Ok(Special::Float(f as f64)) },
            0x1a        => { let f = self.u32(1)?; self.advance(5)?; Ok(Special::Float(f as f64)) },
            0x1b        => { let f = self.u64(1)?; self.advance(9)?; Ok(Special::Float(f as f64)) },
            0x1c..=0x1e => { self.advance(1)?; Ok(Special::Unassigned(b)) },
            0x1f        => { self.advance(1)?; Ok(Special::Break) },
            _           => unreachable!()
        }
    }

    pub fn bool(&mut self) -> Result<bool> {
        self.special()?.unwrap_bool()
    }

    pub fn deserialize<T>(&mut self) -> Result<T>
        where T: Deserialize
    {
        Deserialize::deserialize(self)
    }

    /// Deserialize a value of type `T` and check that there is no
    /// trailing data.
    pub fn deserialize_complete<T>(&mut self) -> Result<T>
        where T: Deserialize
    {
        let v = self.deserialize()?;
        if ! self.is_empty() {
            Err(Error::TrailingData)
        } else {
            Ok(v)
        }
    }
}
impl<'a> From<&'a [u8]> for RawCbor<'a> {
    fn from(bytes: &'a [u8]) -> RawCbor<'a> { RawCbor(bytes) }
}
impl<'a> From<&'a Vec<u8>> for RawCbor<'a> {
    fn from(bytes: &'a Vec<u8>) -> RawCbor<'a> { RawCbor(bytes.as_slice()) }
}
impl<'a, 'b> From<&'b Bytes<'a>> for RawCbor<'a> {
    fn from(bytes: &'b Bytes<'a>) -> RawCbor<'a> { RawCbor(bytes.bytes()) }
}
impl<'a> AsRef<[u8]> for RawCbor<'a> {
    fn as_ref(&self) -> &[u8] { self.0 }
}
impl<'a> Deref for RawCbor<'a> {
    type Target = [u8];
    fn deref(& self) -> &Self::Target { self.0 }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn negative_integer() {
        let vec = vec![0x38, 0x29];
        let mut raw = RawCbor::from(&vec);

        let integer = raw.negative_integer().unwrap();

        assert_eq!(integer, -42);
    }

    #[test]
    fn bytes() {
        let vec = vec![0x52, 0x73, 0x6F, 0x6D, 0x65, 0x20, 0x72, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x20, 0x73, 0x74, 0x72, 0x69, 0x6E, 0x67];
        let mut raw = RawCbor::from(&vec);

        let bytes = raw.bytes().unwrap();
        assert_eq!(&vec[1..], &*bytes);
    }
    #[test]
    fn bytes_empty() {
        let vec = vec![0x40];
        let mut raw = RawCbor::from(&vec);

        let bytes = raw.bytes().unwrap();
        assert!(bytes.is_empty());
    }

    #[test]
    fn text() {
        let vec = vec![0x64, 0x74, 0x65, 0x78, 0x74];
        let mut raw = RawCbor::from(&vec);

        let text = raw.text().unwrap();

        assert_eq!(&text, "text");
    }
    #[test]
    fn text_empty() {
        let vec = vec![0x60];
        let mut raw = RawCbor::from(&vec);

        let text = raw.text().unwrap();

        assert_eq!(&text, "");
    }

    #[test]
    fn array() {
        let vec = vec![0x86, 0,1,2,3,4,5];
        let mut raw = RawCbor::from(&vec);

        let len = raw.array().unwrap();

        assert_eq!(len, Len::Len(6));
        assert_eq!(&*raw, &[0,1,2,3,4,5][..]);

        assert_eq!(0, raw.unsigned_integer().unwrap());
        assert_eq!(1, raw.unsigned_integer().unwrap());
        assert_eq!(2, raw.unsigned_integer().unwrap());
        assert_eq!(3, raw.unsigned_integer().unwrap());
        assert_eq!(4, raw.unsigned_integer().unwrap());
        assert_eq!(5, raw.unsigned_integer().unwrap());
    }
    #[test]
    fn array_empty() {
        let vec = vec![0x80];
        let mut raw = RawCbor::from(&vec);

        let len = raw.array().unwrap();

        assert_eq!(len, Len::Len(0));
        assert_eq!(&*raw, &[][..]);
    }
    #[test]
    fn array_indefinite() {
        let vec = vec![0x9F, 0x01, 0x02, 0xFF];
        let mut raw = RawCbor::from(&vec);

        let len = raw.array().unwrap();

        assert_eq!(len, Len::Indefinite);
        assert_eq!(&*raw, &[0x01, 0x02, 0xFF][..]);

        let i = raw.unsigned_integer().unwrap();
        assert!(i == 1);
        let i = raw.unsigned_integer().unwrap();
        assert!(i == 2);
        assert_eq!(Special::Break, raw.special().unwrap());
    }

    #[test]
    fn complex_array() {
        let vec = vec![0x85, 0x64, 0x69, 0x6F, 0x68, 0x6B, 0x01, 0x20, 0x84, 0, 1, 2, 3, 0x10, /* garbage... */ 0, 1, 2, 3, 4, 5, 6];
        let mut raw = RawCbor::from(&vec);

        let len = raw.array().unwrap();

        assert_eq!(len, Len::Len(5));

        assert_eq!("iohk", &raw.text().unwrap());
        assert_eq!(1, raw.unsigned_integer().unwrap());
        assert_eq!(-1, raw.negative_integer().unwrap());

        let nested_array_len = raw.array().unwrap();
        assert_eq!(nested_array_len, Len::Len(4));
        assert_eq!(0, raw.unsigned_integer().unwrap());
        assert_eq!(1, raw.unsigned_integer().unwrap());
        assert_eq!(2, raw.unsigned_integer().unwrap());
        assert_eq!(3, raw.unsigned_integer().unwrap());

        assert_eq!(0x10, raw.unsigned_integer().unwrap());

        const GARBAGE_LEN : usize = 7;
        assert_eq!(GARBAGE_LEN, raw.len());
    }

    #[test]
    fn map() {
        let vec = vec![0xA2, 0x00, 0x64, 0x74, 0x65, 0x78, 0x74, 0x01, 0x18, 0x2A];
        let mut raw = RawCbor::from(&vec);

        let len = raw.map().unwrap();

        assert_eq!(len, Len::Len(2));

        let k = raw.unsigned_integer().unwrap();
        let v = raw.text().unwrap();
        assert_eq!(0, k);
        assert_eq!("text", &v);

        let k = raw.unsigned_integer().unwrap();
        let v = raw.unsigned_integer().unwrap();
        assert_eq!(1,  k);
        assert_eq!(42, v);
    }

    #[test]
    fn map_empty() {
        let vec = vec![0xA0];
        let mut raw = RawCbor::from(&vec);

        let len = raw.map().unwrap();

        assert_eq!(len, Len::Len(0));
    }

    #[test]
    fn tag() {
        const CBOR : &'static [u8] = &[0xD8, 0x18, 0x52, 0x73, 0x6F, 0x6D, 0x65, 0x20, 0x72, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x20, 0x73, 0x74, 0x72, 0x69, 0x6E, 0x67];
        let mut raw = RawCbor::from(CBOR);

        let tag = raw.tag().unwrap();

        assert_eq!(24, tag);
        let tagged = raw.bytes().unwrap();
        assert_eq!(b"some random string", &*tagged);
    }

    #[test]
    fn tag2() {
        const CBOR : &'static [u8] = &[0x82, 0xd8, 0x18, 0x53, 0x52, 0x73, 0x6f, 0x6d, 0x65, 0x20, 0x72, 0x61, 0x6e, 0x64, 0x6f, 0x6d, 0x20, 0x73, 0x74, 0x72, 0x69, 0x6e, 0x67, 0x1a, 0x71, 0xad, 0x58, 0x36];
        let mut raw = RawCbor::from(CBOR);

        let len = raw.array().unwrap();
        assert_eq!(len, Len::Len(2));

        let tag = raw.tag().unwrap();
        assert!(tag == 24);
        let _ = raw.bytes().unwrap();

        let crc = raw.unsigned_integer().unwrap();
        assert!(crc as u32 == 0x71AD5836);
    }
}
