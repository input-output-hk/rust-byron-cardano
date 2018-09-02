//! CBOR serialisation tooling
use std::io::{Write};

use result::Result;
use types::{Type, Special};
use len::Len;

pub trait Serialize {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>>;
}
impl<'a, T: Serialize> Serialize for &'a T {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.serialize(*self)
    }
}
impl Serialize for u64 {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_unsigned_integer(*self)
    }
}
impl Serialize for u32 {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_unsigned_integer((*self) as u64)
    }
}
impl Serialize for u16 {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_unsigned_integer((*self) as u64)
    }
}
impl Serialize for u8 {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_unsigned_integer((*self) as u64)
    }
}
impl Serialize for bool {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_special(Special::Bool(*self))
    }
}
impl Serialize for String {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_text(self)
    }
}
impl<'a> Serialize for &'a [u8] {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_bytes(self)
    }
}
impl<'a, A, B> Serialize for (&'a A, &'a B)
    where A: Serialize
        , B: Serialize
{
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_array(Len::Len(2))?
                  .serialize(self.0)?
                  .serialize(self.1)
    }
}
impl<'a, A, B, C> Serialize for (&'a A, &'a B, &'a C)
    where A: Serialize
        , B: Serialize
        , C: Serialize
{
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        serializer.write_array(Len::Len(3))?
                  .serialize(self.0)?
                  .serialize(self.1)?
                  .serialize(self.2)
    }
}

impl<T> Serialize for Option<T>
    where T: Serialize
{
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        match self {
            None => serializer.write_array(Len::Len(0)),
            Some(x) => {
                serializer.write_array(Len::Len(1))?.serialize(x)
            }
        }
    }
}

/// helper function to serialise a map of fixed size.
///
/// i.e. the size must be known ahead of time
///
pub fn serialize_fixed_map<'a, C, K, V, W>(data: C, serializer: Serializer<W>) -> Result<Serializer<W>>
    where K: 'a + Serialize
        , V: 'a + Serialize
        , C: Iterator<Item = (&'a K, &'a V)> + ExactSizeIterator
        , W: Write+Sized
{
    let mut serializer = serializer.write_map(Len::Len(data.len() as u64))?;
    for element in data {
        serializer = Serialize::serialize(element.0, serializer)?;
        serializer = Serialize::serialize(element.1, serializer)?;
    }
    Ok(serializer)
}

/// helper function to serialise a collection of T as a fixed number of element
///
/// i.e. the size must be known ahead of time
///
pub fn serialize_fixed_array<'a, C, T, W>(data: C, serializer: Serializer<W>) -> Result<Serializer<W>>
    where T: 'a + Serialize
        , C: Iterator<Item = &'a T> + ExactSizeIterator
        , W: Write+Sized
{
    let mut serializer = serializer.write_array(Len::Len(data.len() as u64))?;
    for element in data {
        serializer = Serialize::serialize(element, serializer)?
    }
    Ok(serializer)
}

/// helper function to serialise a map of indefinite number of elements.
///
pub fn serialize_indefinite_map<'a, C, K, V, W>(data: C, serializer: Serializer<W>) -> Result<Serializer<W>>
    where K: 'a + Serialize
        , V: 'a + Serialize
        , C: Iterator<Item = (&'a K, &'a V)>
        , W: Write+Sized
{
    let mut serializer = serializer.write_map(Len::Indefinite)?;
    for element in data {
        serializer = Serialize::serialize(element.0, serializer)?;
        serializer = Serialize::serialize(element.1, serializer)?;
    }
    serializer.write_special(Special::Break)
}

/// helper function to serialise a collection of T as a indefinite number of element
///
pub fn serialize_indefinite_array<'a, C, T, W>(data: C, serializer: Serializer<W>) -> Result<Serializer<W>>
    where T: 'a + Serialize
        , C: Iterator<Item = &'a T>
        , W: Write+Sized
{
    let mut serializer = serializer.write_array(Len::Indefinite)?;
    for element in data {
        serializer = Serialize::serialize(element, serializer)?
    }
    serializer.write_special(Special::Break)
}

/// helper function to serialise cbor in cbor
///
/// The existence of this function is questionable as it does not make sense, from the
/// CBOR protocol point of view, to encode cbor inside cbor. However it is the way
/// the haskell base code is serialising some objects so we need to comply here too
///
/// This function is a more efficient version of:
///
/// ```
/// # use cbor_event::se::{Serializer, Serialize};
/// let serializer = Serializer::new_vec();
/// let bytes = Serialize::serialize(& 0u32, Serializer::new_vec()).unwrap().finalize();
/// serializer.write_bytes(&bytes).unwrap();
/// ```
///
pub fn serialize_cbor_in_cbor<T, W>(data: T, serializer: Serializer<W>) -> Result<Serializer<W>>
    where T: Serialize
        , W: Write+Sized
{
    serializer.write_bytes(&Serialize::serialize(&data, Serializer::new_vec())?.finalize())
}

// use a default capacity when allocating the Serializer to avoid small reallocation
// at the beginning of the serialisation process as Vec grows by 2, starting from a
// small or an empty serializer will only increase the number of realloc called at
// every _reserve_ calls.
const DEFAULT_CAPACITY : usize = 512;

/// simple CBOR serializer into any
/// [`std::io::Write`](https://doc.rust-lang.org/std/io/trait.Write.html).
///
#[derive(Debug)]
pub struct Serializer<W: Write+Sized>(W);
impl Serializer<Vec<u8>> {
    /// create a new serializer.
    ///
    /// ```
    /// use cbor_event::se::{Serializer};
    ///
    /// let serializer = Serializer::new_vec();
    /// ```
    #[inline]
    pub fn new_vec() -> Self { Serializer::new(Vec::with_capacity(DEFAULT_CAPACITY)) }
}

impl<W: Write+Sized> Serializer<W> {
    #[inline]
    pub fn new(w: W) -> Self { Serializer(w) }

    /// finalize the serializer, returning the serializer bytes
    ///
    /// ```
    /// use cbor_event::se::{Serializer};
    ///
    /// let serializer = Serializer::new_vec();
    ///
    /// let bytes = serializer.finalize();
    ///
    /// # assert!(bytes.is_empty());
    /// ```
    #[inline]
    pub fn finalize(self) -> W { self.0 }

    #[inline]
    fn write_u8(mut self, value: u8) -> Result<Self> {
        self.0.write_all(&[value][..])?;
        Ok(self)
    }

    #[inline]
    fn write_u16(mut self, value: u16) -> Result<Self> {
        self.0.write_all(
            &[ ((value & 0xFF_00) >> 8) as u8
             , ((value & 0x00_FF)     ) as u8
             ][..]
        )?;
        Ok(self)
    }

    #[inline]
    fn write_u32(mut self, value: u32) -> Result<Self> {
        self.0.write_all(
            &[ ((value & 0xFF_00_00_00) >> 24) as u8
             , ((value & 0x00_FF_00_00) >> 16) as u8
             , ((value & 0x00_00_FF_00) >>  8) as u8
             , ((value & 0x00_00_00_FF)      ) as u8
             ][..]
        )?;
        Ok(self)
    }

    #[inline]
    fn write_u64(mut self, value: u64) -> Result<Self> {
        self.0.write_all(
            &[ ((value & 0xFF_00_00_00_00_00_00_00) >> 56) as u8
             , ((value & 0x00_FF_00_00_00_00_00_00) >> 48) as u8
             , ((value & 0x00_00_FF_00_00_00_00_00) >> 40) as u8
             , ((value & 0x00_00_00_FF_00_00_00_00) >> 32) as u8
             , ((value & 0x00_00_00_00_FF_00_00_00) >> 24) as u8
             , ((value & 0x00_00_00_00_00_FF_00_00) >> 16) as u8
             , ((value & 0x00_00_00_00_00_00_FF_00) >>  8) as u8
             , ((value & 0x00_00_00_00_00_00_00_FF)      ) as u8
             ][..]
        )?;
        Ok(self)
    }

    #[inline]
    fn write_type(self, cbor_type: Type, len: u64) -> Result<Self> {
        if len <= super::MAX_INLINE_ENCODING {
            self.write_u8(cbor_type.to_byte(len as u8))
        } else if len < 0x1_00 {
            self.write_u8(cbor_type.to_byte(super::CBOR_PAYLOAD_LENGTH_U8))
                .and_then(|s| s.write_u8(len as u8))
        } else if len < 0x1_00_00 {
            self.write_u8(cbor_type.to_byte(super::CBOR_PAYLOAD_LENGTH_U16))
                .and_then(|s| s.write_u16(len as u16))
        } else if len < 0x1_00_00_00_00 {
            self.write_u8(cbor_type.to_byte(super::CBOR_PAYLOAD_LENGTH_U32))
                .and_then(|s| s.write_u32(len as u32))
        } else {
            self.write_u8(cbor_type.to_byte(super::CBOR_PAYLOAD_LENGTH_U64))
                .and_then(|s| s.write_u64(len))
        }
    }

    /// serialise the given unsigned integer
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::se::{Serializer};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer.write_unsigned_integer(0x12)
    ///     .expect("write a negative integer");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0x12].as_ref());
    /// ```
    pub fn write_unsigned_integer(self, value: u64) -> Result<Self> {
        self.write_type(Type::UnsignedInteger, value)
    }

    /// write a negative integer
    ///
    /// This function fails if one tries to write a non negative value.
    ///
    /// ```
    /// use cbor_event::se::{Serializer};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer.write_negative_integer(-12)
    ///     .expect("write a negative integer");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0x2b].as_ref());
    /// ```
    pub fn write_negative_integer(self, value: i64) -> Result<Self> {
        self.write_type(Type::NegativeInteger, (- value - 1) as u64)
    }

    /// write the given object as bytes
    ///
    /// ```
    /// use cbor_event::se::{Serializer};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer.write_bytes(vec![0,1,2,3])
    ///     .expect("write bytes");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0x44, 0,1,2,3].as_ref());
    /// ```
    pub fn write_bytes<B: AsRef<[u8]>>(self, bytes: B) -> Result<Self> {
        let bytes = bytes.as_ref();
        self.write_type(Type::Bytes, bytes.len() as u64)
            .and_then(|mut s| { s.0.write_all(bytes)?; Ok(s) })
    }

    /// write the given object as text
    ///
    /// ```
    /// use cbor_event::se::{Serializer};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer.write_text(r"hello world")
    ///     .expect("write text");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0x6b, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x77, 0x6F, 0x72, 0x6C, 0x64].as_ref());
    /// ```
    pub fn write_text<S: AsRef<str>>(self, text: S) -> Result<Self> {
        let bytes = text.as_ref().as_bytes();
        self.write_type(Type::Text, bytes.len() as u64)
            .and_then(|mut s| { s.0.write_all(bytes)?; Ok(s) })
    }

    /// start to write an array
    ///
    /// Either you know the length of your array and you can pass it to the funtion
    /// or use an indefinite length.
    ///
    /// - if you set a fixed length of element, you are responsible to set the correct
    ///   amount of elements.
    /// - if you set an indefinite length, you are responsible to write the `Special::Break`
    ///   when your stream ends.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::{se::{Serializer}, Len};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer
    ///     .write_array(Len::Len(2)).expect("write an array")
    ///     .write_text(r"hello").expect("write text")
    ///     .write_text(r"world").expect("write text");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0x82, 0x65, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x65, 0x77, 0x6F, 0x72, 0x6C, 0x64].as_ref());
    /// ```
    ///
    /// ```
    /// use cbor_event::{se::{Serializer}, Len, Special};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer
    ///     .write_array(Len::Indefinite).expect("write an array")
    ///     .write_text(r"hello").expect("write text")
    ///     .write_text(r"world").expect("write text")
    ///     .write_special(Special::Break).expect("write break");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0x9f, 0x65, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x65, 0x77, 0x6F, 0x72, 0x6C, 0x64, 0xff].as_ref());
    /// ```
    ///
    pub fn write_array(self, len: Len) -> Result<Self> {
        match len {
            Len::Indefinite => self.write_u8(Type::Array.to_byte(0x1f)),
            Len::Len(len)   => self.write_type(Type::Array, len as u64),
        }
    }

    /// start to write a map
    ///
    /// Either you know the length of your map and you can pass it to the funtion
    /// or use an indefinite length.
    ///
    /// - if you set a fixed length of element, you are responsible to set the correct
    ///   amount of elements.
    /// - if you set an indefinite length, you are responsible to write the `Special::Break`
    ///   when your stream ends.
    ///
    /// A map is like an array but works by pair of element, so the length is half of the
    /// number of element you are going to write, i.e. the number of pairs, not the number
    /// of elements.
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::{se::{Serializer}, Len};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer
    ///     .write_map(Len::Len(2)).expect("write a map")
    ///     .write_unsigned_integer(1).expect("write unsigned integer")
    ///     .write_text(r"hello").expect("write text")
    ///     .write_unsigned_integer(2).expect("write unsigned integer")
    ///     .write_text(r"world").expect("write text");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0xA2, 01, 0x65, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x02, 0x65, 0x77, 0x6F, 0x72, 0x6C, 0x64].as_ref());
    /// ```
    ///
    /// ```
    /// use cbor_event::{se::{Serializer}, Len, Special};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer
    ///     .write_map(Len::Indefinite).expect("write a map")
    ///     .write_unsigned_integer(1).expect("write unsigned integer")
    ///     .write_text(r"hello").expect("write text")
    ///     .write_unsigned_integer(2).expect("write unsigned integer")
    ///     .write_text(r"world").expect("write text")
    ///     .write_special(Special::Break).expect("write the break");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0xbf, 01, 0x65, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x02, 0x65, 0x77, 0x6F, 0x72, 0x6C, 0x64, 0xff].as_ref());
    /// ```
    ///
    pub fn write_map(self, len: Len) -> Result<Self> {
        match len {
            Len::Indefinite => self.write_u8(Type::Map.to_byte(0x1f)),
            Len::Len(len)   => self.write_type(Type::Map, len as u64),
        }
    }

    /// write a tag
    ///
    /// in cbor a tag should be followed by a tagged object. You are responsible
    /// to making sure you are writing the tagged object just after this
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::{se::{Serializer}, Len};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer
    ///     .write_tag(24).expect("write a tag")
    ///     .write_text(r"hello").expect("write text");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0xd8, 0x18, 0x65, 0x68, 0x65, 0x6C, 0x6C, 0x6F].as_ref());
    /// ```
    ///
    pub fn write_tag(self, tag: u64) -> Result<Self> {
        self.write_type(Type::Tag, tag)
    }

    /// Write a tag that indicates that the following list is a finite
    /// set. See https://www.iana.org/assignments/cbor-tags/cbor-tags.xhtml.
    pub fn write_set_tag(self) -> Result<Self> {
        self.write_type(Type::Tag, 258)
    }

    /// write a special value in cbor
    ///
    /// # Example
    ///
    /// ```
    /// use cbor_event::{se::{Serializer}, Len, Special};
    ///
    /// let serializer = Serializer::new_vec();
    /// let serializer = serializer
    ///     .write_array(Len::Indefinite).expect("write an array")
    ///     .write_special(Special::Bool(false)).expect("write false")
    ///     .write_special(Special::Bool(true)).expect("write true")
    ///     .write_special(Special::Null).expect("write null")
    ///     .write_special(Special::Undefined).expect("write undefined")
    ///     .write_special(Special::Break).expect("write the break");
    ///
    /// # let bytes = serializer.finalize();
    /// # assert_eq!(bytes, [0x9f, 0xf4, 0xf5, 0xf6, 0xf7, 0xff].as_ref());
    /// ```
    pub fn write_special(self, special: Special) -> Result<Self> {
        match special {
            Special::Unassigned(v@0..=0x13) => {
                self.write_u8(Type::Special.to_byte(v))
            },
            Special::Bool(false)   => self.write_u8(Type::Special.to_byte(0x14)),
            Special::Bool(true)    => self.write_u8(Type::Special.to_byte(0x15)),
            Special::Null          => self.write_u8(Type::Special.to_byte(0x16)),
            Special::Undefined     => self.write_u8(Type::Special.to_byte(0x17)),
            Special::Unassigned(v) => {
                self.write_u8(Type::Special.to_byte(0x18))
                    .and_then(|s| s.write_u8(v))
            },
            Special::Float(f)      => {
                unimplemented!("we currently do not support floating point serialisation, cannot serialize: {}", f)
            },
            Special::Break         => self.write_u8(Type::Special.to_byte(0x1f)),
        }
    }

    /// Convenient member function to chain serialisation
    pub fn serialize<T: Serialize>(self, t: &T) -> Result<Self> { Serialize::serialize(t, self) }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn unsigned_integer_0() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_unsigned_integer(0x12)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x12].as_ref());
    }

    #[test]
    fn unsigned_integer_1() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_unsigned_integer(0x20)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x18, 0x20].as_ref());
    }

    #[test]
    fn unsigned_integer_2() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_unsigned_integer(0x2021)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x19, 0x20, 0x21].as_ref());
    }

    #[test]
    fn unsigned_integer_3() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_unsigned_integer(0x20212223)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x1a, 0x20, 0x21, 0x22, 0x23].as_ref());
    }

    #[test]
    fn unsigned_integer_4() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_unsigned_integer(0x2021222324252627)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x1b, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27].as_ref());
    }

    #[test]
    fn negative_integer_0() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_negative_integer(-12)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x2b].as_ref());
    }

    #[test]
    fn negative_integer_1() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_negative_integer(-200)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x38, 0xc7].as_ref());
    }

    #[test]
    fn negative_integer_2() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_negative_integer(-13201)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x39, 0x33, 0x90].as_ref());
    }

    #[test]
    fn negative_integer_3() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_negative_integer(-13201782)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x3a, 0x00, 0xc9, 0x71, 0x75].as_ref());
    }

    #[test]
    fn negative_integer_4() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_negative_integer(-9902201782)
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x3b, 0x00, 0x00, 0x00, 0x02, 0x4E, 0x37, 0x9B, 0xB5].as_ref());
    }

    #[test]
    fn bytes_0() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_bytes(&vec![])
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x40].as_ref());
    }

    #[test]
    fn bytes_1() {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_bytes(&vec![0b101010])
            .expect("write unsigned integer");
        let bytes = serializer.finalize();
        assert_eq!(bytes, [0x41, 0b101010].as_ref());
    }

    fn test_special(cbor_type: Special, result: &[u8]) -> bool {
        let serializer = Serializer::new_vec();
        let serializer = serializer.write_special(cbor_type)
            .expect("serialize a special");
        let bytes = serializer.finalize();
        println!("serializing: {:?}", cbor_type);
        println!("  - expected: {:?}", result);
        println!("  - got:      {:?}", bytes);
        bytes == result
    }

    #[test]
    fn special_false() {
        assert!(test_special(Special::Bool(false), [0xf4].as_ref()))
    }
    #[test]
    fn special_true() {
        assert!(test_special(Special::Bool(true), [0xf5].as_ref()))
    }
    #[test]
    fn special_null() {
        assert!(test_special(Special::Null, [0xf6].as_ref()))
    }
    #[test]
    fn special_undefined() {
        assert!(test_special(Special::Undefined, [0xf7].as_ref()))
    }
    #[test]
    fn special_break() {
        assert!(test_special(Special::Break, [0xff].as_ref()))
    }
    #[test]
    fn special_unassigned() {
        assert!(test_special(Special::Unassigned(0), [0xe0].as_ref()));
        assert!(test_special(Special::Unassigned(1), [0xe1].as_ref()));
        assert!(test_special(Special::Unassigned(10), [0xea].as_ref()));
        assert!(test_special(Special::Unassigned(19), [0xf3].as_ref()));
        assert!(test_special(Special::Unassigned(24), [0xf8, 0x18].as_ref()));
    }
}
