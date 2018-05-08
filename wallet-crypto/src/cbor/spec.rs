//! CBor as specified by the RFC

use std::collections::{BTreeMap, LinkedList};
use std::cmp::{min};
use std::{io, result, fmt};
use util::hex;

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub enum MajorType {
    UINT,
    NINT,
    BYTES,
    TEXT,
    ARRAY,
    MAP,
    TAG,
    T7
}

impl MajorType {
    // serialize a major type in its highest bit form
    fn to_byte(self, r: u8) -> u8 {
        use self::MajorType::*;
        assert!(r <= 0b0001_1111);

        r | match self {
            UINT  => 0b0000_0000,
            NINT  => 0b0010_0000,
            BYTES => 0b0100_0000,
            TEXT  => 0b0110_0000,
            ARRAY => 0b1000_0000,
            MAP   => 0b1010_0000,
            TAG   => 0b1100_0000,
            T7    => 0b1110_0000
        }
    }

    fn from_byte(byte: u8) -> Self {
        use self::MajorType::*;
        match byte & 0b1110_0000 {
            0b0000_0000 => UINT,
            0b0010_0000 => NINT,
            0b0100_0000 => BYTES,
            0b0110_0000 => TEXT,
            0b1000_0000 => ARRAY,
            0b1010_0000 => MAP,
            0b1100_0000 => TAG,
            0b1110_0000 => T7,
            _           => panic!("the impossible happened!")
        }
    }
}

#[test]
fn major_type_byte_encoding() {
    for i in 0b0000_0000..0b0001_1111 {
        assert!(MajorType::UINT  == MajorType::from_byte(MajorType::to_byte(MajorType::UINT,  i)));
        assert!(MajorType::NINT  == MajorType::from_byte(MajorType::to_byte(MajorType::NINT,  i)));
        assert!(MajorType::BYTES == MajorType::from_byte(MajorType::to_byte(MajorType::BYTES, i)));
        assert!(MajorType::TEXT  == MajorType::from_byte(MajorType::to_byte(MajorType::TEXT,  i)));
        assert!(MajorType::ARRAY == MajorType::from_byte(MajorType::to_byte(MajorType::ARRAY, i)));
        assert!(MajorType::MAP   == MajorType::from_byte(MajorType::to_byte(MajorType::MAP,   i)));
        assert!(MajorType::TAG   == MajorType::from_byte(MajorType::to_byte(MajorType::TAG,   i)));
        assert!(MajorType::T7    == MajorType::from_byte(MajorType::to_byte(MajorType::T7,    i)));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReaderError {
    NotEnoughBytes,
    EndOfBuffer
}
impl From<ReaderError> for Error {
    fn from(e: ReaderError) -> Error { Error::ReaderError(e) }
}


#[derive(Clone, PartialEq, Eq)]
pub enum Error {
    ExpectedU8,
    ExpectedU16,
    ExpectedU32,
    ExpectedU64,
    ExpectedI8,
    ExpectedI16,
    ExpectedI32,
    ExpectedI64,
    ExpectedBytes,
    ExpectedText,
    ExpectedArray,
    ExpectedObject,
    ExpectedTag,
    ExpectedT7,
    ArrayUndefinedIndex(usize),
    ObjectUndefinedElement(ObjectKey),
    InvalidSize(usize),
    NotOneOf(&'static [Value]),
    InvalidSumtype(u64),
    InvalidTag(u64),
    InvalidValue(Box<Value>),
    UnparsedValues,
    Between(u64, u64),
    ReaderError(ReaderError),
    UnknownMinorType(u8),
    ExpectedValue,

    EmbedWith(&'static str, Box<Error>)
}
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        match self {
            &Error::ExpectedU8 => write!(f, "Expected U8"),
            &Error::ExpectedU16 => write!(f, "Expected U16"),
            &Error::ExpectedU32 => write!(f, "Expected U32"),
            &Error::ExpectedU64 => write!(f, "Expected U64"),
            &Error::ExpectedI8 => write!(f, "Expected I8"),
            &Error::ExpectedI16 => write!(f, "Expected I16"),
            &Error::ExpectedI32 => write!(f, "Expected I32"),
            &Error::ExpectedI64 => write!(f, "Expected I64"),
            &Error::ExpectedBytes => write!(f, "Expected Bytes"),
            &Error::ExpectedText => write!(f, "Expected UTF8 String"),
            &Error::ExpectedArray => write!(f, "Expected Array"),
            &Error::ExpectedObject => write!(f, "Expected Object"),
            &Error::ExpectedTag => write!(f, "Expected Tag"),
            &Error::ExpectedT7 => write!(f, "Expected T7"),
            &Error::ArrayUndefinedIndex(index) => write!(f, "Index {:?} undefined", index),
            &Error::ObjectUndefinedElement(ref ok) => write!(f, "Key {:?} undefined", ok),
            &Error::InvalidSize(size) => write!(f, "invalid size, expected {:?}", size),
            &Error::NotOneOf(val) => write!(f, "Expected one of: {:?}", val),
            &Error::InvalidSumtype(index) => write!(f, "invalid sumtype index {:?}", index),
            &Error::InvalidTag(tag) => write!(f, "expected tag id {:?}", tag),
            &Error::InvalidValue(ref val) => write!(f, "expected value {:?}", val),
            &Error::UnparsedValues => write!(f, "unparsed values"),
            &Error::Between(min, max) => write!(f, "expected between [{:?}..{:?}]", min, max),
            &Error::ReaderError(ref err) => write!(f, "reader error: {:?}", err),
            &Error::UnknownMinorType(t) => write!(f, "unknown minor type: {:X}", t),
            &Error::ExpectedValue => write!(f, "Cannot parse the value"),
            &Error::EmbedWith(ref msg, ref embedded) => {
                write!(f, "{}\n", msg)?;
                write!(f, "  {:?}", *embedded)
            }
        }
    }
}

pub type Result<V> = result::Result<V, (Value, Error)>;
pub trait ExtendedResult {
    fn embed(self, &'static str) -> Self;
    fn u64(v: u64, err: Error) -> Self;
    fn i64(v: i64, err: Error) -> Self;
    fn text(v: String, err: Error) -> Self;
    fn bytes(v: Bytes, err: Error) -> Self;
    fn array(v: Vec<Value>, err: Error) -> Self;
    fn iarray(v: LinkedList<Value>, err: Error) -> Self;
    fn object(v: BTreeMap<ObjectKey, Value>, err: Error) -> Self;
    fn tag(tag: u64, v: Box<Value>, err:Error) -> Self;
}
impl<V> ExtendedResult for Result<V> {
    fn embed(self, msg: &'static str) -> Self {
        self.or_else(|(v, err)| {
            Err((v, Error::EmbedWith(msg, Box::new(err))))
        })
    }
    fn u64(v: u64, err: Error) -> Self { Err((Value::U64(v), err)) }
    fn i64(v: i64, err: Error) -> Self { Err((Value::I64(v), err)) }
    fn text(v: String, err: Error) -> Self { Err((Value::Text(v), err)) }
    fn bytes(v: Bytes, err: Error) -> Self { Err((Value::Bytes(v), err)) }
    fn array(v: Vec<Value>, err: Error) -> Self { Err((Value::Array(v), err)) }
    fn iarray(v: LinkedList<Value>, err: Error) -> Self { Err((Value::IArray(v), err)) }
    fn object(v: BTreeMap<ObjectKey, Value>, err: Error) -> Self { Err((Value::Object(v), err)) }
    fn tag(tag: u64, v: Box<Value>, err:Error) -> Self {
        Err((Value::Tag(tag, v), err))
    }
}
pub fn array_decode_elem<T>(mut array: Vec<Value>, index: usize) -> Result<(Vec<Value>, T)>
    where T: CborValue
{
    match array.get(index).map(|v| v.clone()) {
        Some(value) => {
            array.remove(index);
            CborValue::decode(value)
                .map(|t| (array, t) )
                .embed("while decoding array's element")
        },
        None => { Result::array(array, Error::ArrayUndefinedIndex(index)) }
    }
}
pub fn object_decode_elem<T>(mut object: BTreeMap<ObjectKey, Value>, index: ObjectKey) -> Result<(BTreeMap<ObjectKey, Value>, T)>
    where T: CborValue
{
    match object.remove(&index) {
        Some(value) => {
            CborValue::decode(value)
                .embed("while decoding object's element")
                .and_then(|t| {Ok((object, t))})
        },
        None => { Result::object(object, Error::ObjectUndefinedElement(index)) }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    U64(u64),
    I64(i64),
    Bytes(Bytes),
    Text(String),
    Array(Vec<Value>),
    ArrayStart,
    IArray(LinkedList<Value>),
    Object(BTreeMap<ObjectKey, Value>),
    Tag(u64, Box<Value>),
    Break,
    Null,
}
impl Value {
    pub fn u64(self) -> Result<u64> {
        match self {
            Value::U64(v) => Ok(v),
            v             => Err((v, Error::ExpectedU64))
        }
    }
    pub fn i64(self) -> Result<i64> {
        match self {
            Value::I64(v) => Ok(v),
            v              => Err((v, Error::ExpectedI64))
        }
    }
    pub fn text(self) -> Result<String> {
        match self {
            Value::Text(v) => Ok(v),
            v              => Err((v, Error::ExpectedText))
        }
    }
    pub fn bytes(self) -> Result<Bytes> {
        match self {
            Value::Bytes(v) => Ok(v),
            v               => Err((v, Error::ExpectedBytes))
        }
    }
    pub fn array(self) -> Result<Vec<Value>> {
        match self {
            Value::Array(v) => Ok(v),
            v               => Err((v, Error::ExpectedArray))
        }
    }
    pub fn iarray(self) -> Result<LinkedList<Value>> {
        match self {
            Value::IArray(v) => Ok(v),
            v                => Err((v, Error::ExpectedArray))
        }
    }
    pub fn object(self) -> Result<BTreeMap<ObjectKey, Value>> {
        match self {
            Value::Object(v) => Ok(v),
            v                => Err((v, Error::ExpectedObject))
        }
    }
    pub fn tag(self) -> Result<(u64, Box<Value>)> {
        match self {
            Value::Tag(t, v) => Ok((t, v)),
            v                => Err((v, Error::ExpectedTag))
        }
    }

    pub fn decode<T>(self) -> Result<T>
        where T: CborValue
    {
        CborValue::decode(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectKey {
    Integer(u64),
    Bytes(Bytes),
    Text(String),
    // Bool(bool)
}

pub trait CborValue: Sized {
    fn encode(&self) -> Value;
    fn decode(v: Value) -> Result<Self>;
}
impl CborValue for Value {
    fn encode(&self) -> Value { self.clone() }
    fn decode(v: Value) -> Result<Self> { Ok(v) }
}
impl CborValue for u8 {
    fn encode(&self)  -> Value { Value::U64(*self as u64) }
    fn decode(v: Value) -> Result<Self> {
        v.u64().and_then(|v| {
            if v < 0x100 { Ok(v as Self) } else { Result::u64(v, Error::ExpectedU8) }
        }).embed("while decoding `u8'")
    }
}
impl CborValue for u16 {
    fn encode(&self)  -> Value { Value::U64(*self as u64) }
    fn decode(v: Value) -> Result<Self> {
        v.u64().and_then(|v| {
            if v < 0x10000 { Ok(v as Self) } else { Result::u64(v, Error::ExpectedU16) }
        }).embed("while decoding `u16'")
    }
}
impl CborValue for u32 {
    fn encode(&self)  -> Value { Value::U64(*self as u64) }
    fn decode(v: Value) -> Result<Self> {
        v.u64().and_then(|v| {
            if v < 0x100000000 { Ok(v as Self) } else { Result::u64(v, Error::ExpectedU32) }
        }).embed("while decoding `u32'")
    }
}
impl CborValue for u64 {
    fn encode(&self)  -> Value { Value::U64(*self) }
    fn decode(v: Value) -> Result<Self> {
        v.u64().embed("while decoding `u64'")
    }
}
impl CborValue for String {
    fn encode(&self)  -> Value { Value::Text(self.clone()) }
    fn decode(v: Value) -> Result<Self> {
        v.text().embed("while decoding `text'")
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Bytes(Vec<u8>);
impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl AsRef<[u8]> for Bytes { fn as_ref(&self) -> &[u8] { self.0.as_ref() } }
impl Bytes {
    pub fn new(bytes: Vec<u8>) -> Self { Bytes(bytes) }
    pub fn from_slice(bytes: &[u8]) -> Self { Bytes::new(bytes.iter().cloned().collect()) }
    pub fn len(&self) -> usize { self.0.len() }
    pub fn to_vec(self) -> Vec<u8> { self.0 }
}
impl CborValue for Bytes {
    fn encode(&self)  -> Value { Value::Bytes(self.clone()) }
    fn decode(v: Value) -> Result<Self> { v.bytes() }
}
impl<T> CborValue for Vec<T> where T: CborValue {
    fn encode(&self) -> Value {
        let mut vec = vec![];
        for i in self.iter() {
            let v = CborValue::encode(i);
            vec.push(v);
        }
        Value::Array(vec)
    }
    fn decode(value: Value) -> Result<Self> {
        value.array().and_then(|array| {
            let mut vec = vec![];
            for i in array.iter() {
                let v = CborValue::decode(i.clone())?;
                vec.push(v);
            }
            Ok(vec)
        })
    }
}
impl<T> CborValue for LinkedList<T> where T: CborValue {
    fn encode(&self) -> Value {
        let mut l = LinkedList::new();
        for i in self.iter() {
            let v = CborValue::encode(i);
            l.push_back(v);
        }
        Value::IArray(l)
    }
    fn decode(value: Value) -> Result<Self> {
        value.iarray().and_then(|list| {
            let mut r = LinkedList::new();
            for i in list.iter() {
                let v = CborValue::decode(i.clone())?;
                r.push_back(v);
            }
            Ok(r)
        })
    }
}
impl<T> CborValue for Option<T> where T: CborValue {
    fn encode(&self)  -> Value {
        match self {
            &None => Value::Null,
            &Some(ref v) => CborValue::encode(v)
        }
    }
    fn decode(value: Value) -> Result<Self> {
        CborValue::decode(value).map(|v| {Some(v)}).or(Ok(None))
    }
}
impl<A, B> CborValue for (A, B)
    where A: CborValue
        , B: CborValue
{
    fn encode(&self)  -> Value {
        Value::Array(
            vec![ CborValue::encode(&self.0)
                , CborValue::encode(&self.1)
                ]
        )
    }
    fn decode(v: Value) -> Result<Self> {
        v.array().and_then(|tuple| {
            let (tuple, x) = array_decode_elem(tuple, 0).embed("while decoding first's element of the tuple")?;
            let (tuple, y) = array_decode_elem(tuple, 0).embed("while decoding second's element of the tuple")?;
            if tuple.len() != 0 {
                Result::array(tuple, Error::UnparsedValues)
            } else {
                Ok((x,y))
            }
        })
    }
}
impl<A, B, C> CborValue for (A, B, C)
    where A: CborValue
        , B: CborValue
        , C: CborValue
{
    fn encode(&self)  -> Value {
        Value::Array(
            vec![ CborValue::encode(&self.0)
                , CborValue::encode(&self.1)
                , CborValue::encode(&self.2)
                ]
        )
    }
    fn decode(v: Value) -> Result<Self> {
        v.array().and_then(|tuple| {
            let (tuple, x) = array_decode_elem(tuple, 0).embed("while decoding first's element of the tuple")?;
            let (tuple, y) = array_decode_elem(tuple, 0).embed("while decoding second's element of the tuple")?;
            let (tuple, z) = array_decode_elem(tuple, 0).embed("while decoding third's element of the tuple")?;
            if tuple.len() != 0 {
                Result::array(tuple, Error::UnparsedValues)
            } else {
                Ok((x,y,z))
            }
        })
    }
}

const MAX_INLINE_ENCODING : u8 = 23;
const CBOR_PAYLOAD_LENGTH_U8 : u8 = 24;
const CBOR_PAYLOAD_LENGTH_U16 : u8 = 25;
const CBOR_PAYLOAD_LENGTH_U32 : u8 = 26;
const CBOR_PAYLOAD_LENGTH_U64 : u8 = 27;

/// convenient macro to get the given bytes of the given value
///
/// does all the job: Big Endian, bit shift and convertion
macro_rules! byte_slice {
    ($value:ident, $shift:expr) => ({
        ($value >> $shift) as u8
    });
}

/// convenient function to encode a `CborValue` object to a byte array
///
pub fn encode_to_cbor<V>(v: &V) -> io::Result<Vec<u8>>
    where V: CborValue
{
    let mut encoder = Encoder::new(vec![]);

    encoder.write(&CborValue::encode(v))?;

    Ok(encoder.writer)
}

/// convenient function to decode the given bytes from cbor encoding
///
pub fn decode_from_cbor<V>(buf: &[u8]) -> Result<V>
    where V: CborValue
{
    let mut reader = vec![]; reader.extend_from_slice(buf);
    let mut decoder = Decoder::new(reader);

    match decoder.value() {
        Err(err) => Err((Value::Null, err)),
        Ok(None) => {
            Err((Value::Null, Error::ExpectedValue))
        },
        Ok(Some(value)) => {
            CborValue::decode(value)
        }
    }
}

/// create CBOR serialiser
pub struct Encoder<W> {
    writer: W
}
impl<W> Encoder<W> where W: io::Write {
    pub fn new(w: W) -> Self { Encoder { writer: w } }

    fn write_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes)
    }

    fn write_header_u8(&mut self, ty: MajorType, v: u8) -> io::Result<()> {
        self.write_bytes(&
            [ ty.to_byte(CBOR_PAYLOAD_LENGTH_U8)
            , v
            ]
        )
    }

    fn write_header_u16(&mut self, ty: MajorType, v: u16) -> io::Result<()> {
        self.write_bytes(&
            [ ty.to_byte(CBOR_PAYLOAD_LENGTH_U16)
            , byte_slice!(v, 8)
            , byte_slice!(v, 0)
            ]
        )
    }
    fn write_header_u32(&mut self, ty: MajorType, v: u32) -> io::Result<()> {
        self.write_bytes(&
            [ ty.to_byte(CBOR_PAYLOAD_LENGTH_U32)
            , byte_slice!(v, 24)
            , byte_slice!(v, 16)
            , byte_slice!(v, 8)
            , byte_slice!(v, 0)
            ]
        )
    }
    fn write_header_u64(&mut self, ty: MajorType, v: u64) -> io::Result<()> {
        self.write_bytes(&
            [ ty.to_byte(CBOR_PAYLOAD_LENGTH_U64)
            , byte_slice!(v, 56)
            , byte_slice!(v, 48)
            , byte_slice!(v, 40)
            , byte_slice!(v, 32)
            , byte_slice!(v, 24)
            , byte_slice!(v, 16)
            , byte_slice!(v, 8)
            , byte_slice!(v, 0)
            ]
        )
    }

    fn write_header(&mut self, ty: MajorType, nb_elems: u64) -> io::Result<()> {
        if nb_elems <= (MAX_INLINE_ENCODING as u64) {
            self.write_bytes(&[ty.to_byte(nb_elems as u8)])
        } else {
            if nb_elems < 0x100 {
                self.write_header_u8(ty, nb_elems as u8)
            } else if nb_elems < 0x10000 {
                self.write_header_u16(ty, nb_elems as u16)
            } else if nb_elems < 0x100000000 {
                self.write_header_u32(ty, nb_elems as u32)
            } else {
                self.write_header_u64(ty, nb_elems as u64)
            }
        }
    }

    fn write_bs(&mut self, v: &Bytes) -> io::Result<()> {
        self.write_header(MajorType::BYTES, v.len() as u64)?;
        self.write_bytes(v.as_ref())
    }

    fn write_text(&mut self, s: &String) -> io::Result<()> {
        let v = s.as_bytes();
        self.write_header(MajorType::TEXT, v.len() as u64)?;
        self.write_bytes(v)
    }


    fn write_array(&mut self, v: &Vec<Value>) -> io::Result<()> {
        self.write_header(MajorType::ARRAY, v.len() as u64)?;
        for e in v.iter() { self.write(e)?; }
        Ok(())
    }

    fn write_object(&mut self, v: &BTreeMap<ObjectKey, Value>) -> io::Result<()> {
        self.write_header(MajorType::MAP, v.len() as u64)?;
        for e in v.iter() { self.write_key(e.0)?; self.write(e.1)?; }
        Ok(())
    }

    fn start_indefinite(&mut self, mt: MajorType) -> io::Result<()> {
        self.write_bytes(&[mt.to_byte(0x1F)])
    }

    fn write_iarray(&mut self, v: &LinkedList<Value>) -> io::Result<()> {
        self.start_indefinite(MajorType::ARRAY)?;
        for e in v.iter() { self.write(e)?; }
        self.write_bytes(&[0xFF]) // add the break
    }

    pub fn write(&mut self, value: &Value) -> io::Result<()> {
        match value {
            &Value::U64(ref v)    => self.write_header(MajorType::UINT, *v),
            &Value::I64(ref v)    => self.write_header(MajorType::NINT, *v as u64),
            &Value::Bytes(ref v)  => self.write_bs(v),
            &Value::Text(ref v)   => self.write_text(v),
            &Value::Array(ref v)  => self.write_array(&v),
            &Value::IArray(ref v) => self.write_iarray(&v),
            &Value::ArrayStart    => self.start_indefinite(MajorType::ARRAY),
            &Value::Object(ref v) => self.write_object(&v),
            &Value::Tag(ref t, ref v) => {
                self.write_header(MajorType::TAG, *t)?;
                self.write(v.as_ref())
            },
            &Value::Break        => self.write_bytes(&[0xFF]),
            &Value::Null         => Ok(()),
        }
    }
    pub fn write_key(&mut self, key: &ObjectKey) -> io::Result<()> {
        match key {
            &ObjectKey::Integer(ref v) => self.write_header(MajorType::UINT, *v),
            &ObjectKey::Bytes(ref v)   => self.write_bs(v),
            &ObjectKey::Text(ref v)    => self.write_text(v),
        }
    }
}

pub trait Read {
    fn next(&mut self) -> result::Result<u8, ReaderError>;
    fn peek(&self) -> Option<u8>;
    fn discard(&mut self);
    fn read(&mut self, len: usize) -> Vec<u8>;
    fn read_into(&mut self, buf: &mut [u8]) -> usize;
}
impl Read for Vec<u8> {
    fn next(&mut self) -> result::Result<u8, ReaderError> {
        if self.len() > 0 { Ok(self.remove(0)) } else { Err(ReaderError::NotEnoughBytes) }
    }
    fn peek(&self) -> Option<u8> {
        if self.len() > 0 { Some(self[0]) } else { None }
    }
    fn discard(&mut self) { if self.len() > 0 { self.remove(0); } }
    fn read(&mut self, sz: usize) -> Vec<u8> {
        let len = min(self.len(), sz);
        if len == 0 { return vec![]; }

        let mut v = vec![];
        v.extend_from_slice(&self[..len]);
        for _ in 0..len { self.discard(); }

        v
    }
    fn read_into(&mut self, buf: &mut [u8]) -> usize {
        let len = min(self.len(), buf.len());
        if len == 0 { return 0; }

        buf[..len].clone_from_slice(self.as_ref());
        for _ in 0..len { self.discard(); }

        len
    }
}

/// create CBOR serialiser
pub struct Decoder<R> {
    reader: R
}
impl<R> Decoder<R> where R: Read {
    pub fn new(reader: R) -> Self { Decoder { reader: reader } }

    fn consume(&mut self) { self.reader.discard() }

    pub fn peek_type(&mut self) -> Option<MajorType> {
        self.reader.peek().map(MajorType::from_byte)
    }

    fn u8(&mut self) -> result::Result<u64, Error> {
        let b = self.reader.next()?;
        Ok(b as u64)
    }
    fn u16(&mut self) ->result::Result<u64, Error> { 
        let b1 = self.u8()?;
        let b2 = self.u8()?;
        Ok(b1 << 8 | b2)
    }
    fn u32(&mut self) -> result::Result<u64, Error> {
        let b1 = self.u8()?;
        let b2 = self.u8()?;
        let b3 = self.u8()?;
        let b4 = self.u8()?;
        Ok(b1 << 24 | b2 << 16 | b3 << 8 | b4)
    }
    fn u64(&mut self) -> result::Result<u64, Error> {
        let b1 = self.u8()?;
        let b2 = self.u8()?;
        let b3 = self.u8()?;
        let b4 = self.u8()?;
        let b5 = self.u8()?;
        let b6 = self.u8()?;
        let b7 = self.u8()?;
        let b8 = self.u8()?;
        Ok(b1 << 56 | b2 << 48 | b3 << 40 | b4 << 32 | b5 << 24 | b6 << 16 | b7 << 8 | b8)
    }

    fn get_minor(&mut self) -> Option<u8> {
        self.reader.peek().map(|b| { b & 0b0001_1111 } )
    }

    fn get_minor_type(&mut self) -> result::Result<Option<u64>, Error> {
        let b = match self.get_minor() {
            None => return Ok(None) ,
            Some(b) => b,
        };
        match b & 0b0001_1111 {
            0x00...0x17 => { self.consume(); Ok(Some(b as u64)) },
            0x18        => { self.consume(); self.u8().map(|v| Some(v)) },
            0x19        => { self.consume(); self.u16().map(|v| Some(v)) },
            0x1a        => { self.consume(); self.u32().map(|v| Some(v)) },
            0x1b        => { self.consume(); self.u64().map(|v| Some(v)) },
            0x1c...0x1e => Err(Error::UnknownMinorType(b & 0b0001_1111)),
            0x1f        => Ok(None),
            _           => Err(Error::UnknownMinorType(b))
        }
    }

    fn key(&mut self) -> result::Result<Option<ObjectKey>, Error> {
        let ty = match self.peek_type() {
            Some(b) => b,
            None => return Ok(None)
        };
        match ty {
            MajorType::UINT => { self.get_minor_type().map(|opt| opt.map(ObjectKey::Integer)) },
            MajorType::BYTES => {
                let len = match self.get_minor_type()? {
                    Some(b) => b,
                    None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes))
                };
                let buf = self.reader.read(len as usize);
                if len as usize != buf.len() {
                    Err(Error::ReaderError(ReaderError::NotEnoughBytes))
                } else {
                    Ok(Some(ObjectKey::Bytes(Bytes::new(buf)) ))
                }
            },
            MajorType::TEXT => {
                let len = match self.get_minor_type()? {
                    Some(b) => b,
                    None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes))
                };
                let buf = self.reader.read(len as usize);
                if len as usize != buf.len() {
                    Err(Error::ReaderError(ReaderError::NotEnoughBytes))
                } else {
                    Ok(String::from_utf8(buf).ok().map(|s| ObjectKey::Text(s)))
                }
            },
            _ => {
                error!("Expected A {{UINT, BYTES}}, received: {:?}", ty);
                Err(Error::ExpectedU64)
            },
        }
    }

    pub fn value(&mut self) -> result::Result<Option<Value>, Error> {
        let ty = match self.peek_type() {
            None => return Ok(None),
            Some(b) => b
        };
        match ty {
            MajorType::UINT  => { self.get_minor_type().map(|opt| opt.map(Value::U64)) },
            MajorType::NINT  => { self.get_minor_type().map(|opt| opt.map(|v| Value::I64(v as i64))) },
            MajorType::BYTES => {
                let len = match self.get_minor_type()? {
                    Some(b) => b,
                    None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes))
                };
                let buf = self.reader.read(len as usize);
                if len as usize != buf.len() {
                    Err(Error::ReaderError(ReaderError::NotEnoughBytes))
                } else {
                    Ok(Some(Value::Bytes(Bytes::new(buf)) ))
                }
            },
            MajorType::TEXT  => {
                let len = match self.get_minor_type()? {
                    Some(b) => b,
                    None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes))
                };
                let buf = self.reader.read(len as usize);
                if len as usize != buf.len() {
                    Err(Error::ReaderError(ReaderError::NotEnoughBytes))
                } else {
                    Ok(String::from_utf8(buf).ok().map(|s| Value::Text(s)))
                }
            },
            MajorType::ARRAY => {
                let maybe_len = self.get_minor_type()?;
                match maybe_len {
                    None      => {
                        if self.get_minor() == Some(0x1F) {
                            // this is an Indefinite array
                            let mut array = LinkedList::new();
                            // consume the minor type
                            self.consume();
                            loop {
                                let val = match self.value()? {
                                    None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes)),
                                    Some(v) => v,
                                };
                                if val == Value::Break { break; }
                                array.push_back(val);
                            }
                            Ok(Some(Value::IArray(array)))
                        } else {
                            Err(Error::ExpectedT7)
                        }
                    },
                    Some(len) => {
                        let mut array = vec![];
                        for _ in 0..len {
                            match self.value()? {
                                None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes)),
                                Some(v) => array.push(v)
                            }
                         }
                        Ok(Some(Value::Array(array)))
                    }
                }
            },
            MajorType::MAP => {
                let maybe_len = self.get_minor_type()?;
                match maybe_len {
                    None      => {
                        if self.get_minor() == Some(0x1F) {
                           let mut map = BTreeMap::new();
                            // consume the minor type
                            self.consume();
                            loop {
                                let k = match self.key()? {
                                    None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes)),
                                    Some(k) => k
                                };
                                let v = match self.value()? {
                                    None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes)),
                                    Some(v) => v
                                };
                                if v == Value::Break { break; }
                                map.insert(k, v);
                            }
                            Ok(Some(Value::Object(map)))
                        } else {
                            Err(Error::ExpectedT7)
                        }
                    },
                    Some(len) => {
                        let mut map = BTreeMap::new();
                        for _ in 0..len {
                            let k = match self.key()? {
                                None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes)),
                                Some(k) => k
                            };
                            let v = match self.value()? {
                                None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes)),
                                Some(v) => v
                            };
                            map.insert(k, v);
                        }
                        Ok(Some(Value::Object(map)))
                    }
                }
            },
            MajorType::TAG => {
                let tag = match self.get_minor_type()? {
                    None => return Err(Error::ExpectedTag),
                    Some(t) => t
                };
                let obj = match self.value()? {
                    None => return Err(Error::ReaderError(ReaderError::NotEnoughBytes)),
                    Some(v) => v
                };
                Ok(Some(Value::Tag(tag, Box::new(obj))))
            },
            MajorType::T7 => {
                let v = self.get_minor();
                match v {
                    Some(0x1f) => { self.consume(); Ok(Some(Value::Break)) },
                    _          => { self.consume(); Ok(Some(Value::Null)) },
                }
            }
        }
    }
}

/*
pub struct Indefinite<E>(E);
impl<W: io::Write> Indefinite<Encoder<W>> {
    pub fn start_array(e: Encoder<W>) -> io::Result<Self> {
        let mut encoder = e;
        encoder.write_bytes(&[0x9F])?;
        Ok(Indefinite(encoder))
    }

    pub fn write(& mut self, value: &Value) -> io::Result<()> { self.0.write(value) }

    pub fn stop_indefinite(self) -> io::Result<Encoder<W>> {
        let mut encoder = self.0;
        encoder.write_bytes(&[0xFF])?;
        Ok(encoder)
    }
}

impl<R: Read> Indefinite<Decoder<R>> {
    // start array, return the decoder if this is not a start of a array...
    // otherwise returns the new Indefinite<Decoder<R>>
    pub fn start_array(e: Decoder) -> Either<Decoder<R>, Self>;
    // try to read a value `MajorType` that has been read that is ont a none value
    pub fn read(&mut self) -> Either<MajorType, Value>;
    //
    pub fn break(self) -> Either<Self, Decoder<R>>
}
*/
