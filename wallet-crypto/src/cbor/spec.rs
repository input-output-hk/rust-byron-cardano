//! CBor as specified by the RFC

use std::collections::{BTreeMap};
use std::cmp::{min};
use std::io;

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

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    U64(u64),
    I64(i64),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    ArrayStart,
    Object(BTreeMap<ObjectKey, Value>),
    Tag(u64, Box<Value>),
    Break,
    Null,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObjectKey {
    Integer(u64)
}

pub trait CborValue: Sized {
    fn encode(&self) -> Value;
    fn decode(v: &Value) -> Option<Self>;
}
impl CborValue for Value {
    fn encode(&self) -> Value { self.clone() }
    fn decode(v: &Value) -> Option<Self> { Some(v.clone()) }
}
impl CborValue for u8 {
    fn encode(&self)  -> Value { Value::U64(*self as u64) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::U64(ref v) => { if *v < 0x100 { Some(*v as Self) } else { None } }
            _ => None
        }
    }
}
impl CborValue for u16 {
    fn encode(&self)  -> Value { Value::U64(*self as u64) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::U64(ref v) => { if *v < 0x10000 { Some(*v as Self) } else { None } }
            _ => None
        }
    }
}
impl CborValue for u32 {
    fn encode(&self)  -> Value { Value::U64(*self as u64) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::U64(ref v) => { if *v < 0x100000000 { Some(*v as Self) } else { None } }
            _ => None
        }
    }
}
impl CborValue for u64 {
    fn encode(&self)  -> Value { Value::U64(*self) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::U64(ref v) => Some(*v),
            _ => None
        }
    }
}
impl CborValue for i8 {
    fn encode(&self)  -> Value { Value::I64(*self as i64) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::I64(ref v) => { if *v < 0x100 { Some(*v as Self) } else { None } }
            _ => None
        }
    }
}
impl CborValue for i16 {
    fn encode(&self)  -> Value { Value::I64(*self as i64) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::I64(ref v) => { if *v < 0x10000 { Some(*v as Self) } else { None } }
            _ => None
        }
    }
}
impl CborValue for i32 {
    fn encode(&self)  -> Value { Value::I64(*self as i64) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::I64(ref v) => { if *v < 0x100000000 { Some(*v as Self) } else { None } }
            _ => None
        }
    }
}
impl CborValue for i64 {
    fn encode(&self)  -> Value { Value::I64(*self) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::I64(ref v) => Some(*v),
            _ => None
        }
    }
}
impl CborValue for Vec<u8> {
    fn encode(&self)  -> Value { Value::Bytes(self.clone()) }
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::Bytes(ref v) => Some(v.clone()),
            _ => None
        }
    }
}
impl<T> CborValue for Option<T> where T: CborValue {
    fn encode(&self)  -> Value {
        match self {
            &None => Value::Null,
            &Some(ref v) => CborValue::encode(v)
        }
    }
    fn decode(v: &Value) -> Option<Self> {
        let v = CborValue::decode(v)?;
        Some(v)
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
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::Array(ref v) => {
                if (v.len() != 2) { return None; }
                let x = CborValue::decode(&v[0])?;
                let y = CborValue::decode(&v[1])?;
                Some((x,y))
            },
            _ => None
        }
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
    fn decode(v: &Value) -> Option<Self> {
        match v {
            &Value::Array(ref v) => {
                if (v.len() != 3) { return None; }
                let x = CborValue::decode(&v[0])?;
                let y = CborValue::decode(&v[1])?;
                let z = CborValue::decode(&v[2])?;
                Some((x,y,z))
            },
            _ => None
        }
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
pub fn decode_from_cbor<V>(buf: &[u8]) -> Option<V>
    where V: CborValue
{
    let mut reader = vec![]; reader.extend_from_slice(buf);
    let mut decoder = Decoder::new(reader);

    let value = decoder.value()?;
    CborValue::decode(&value)
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

    fn write_bs(&mut self, v: &Vec<u8>) -> io::Result<()> {
        self.write_header(MajorType::BYTES, v.len() as u64)?;
        self.write_bytes(v.as_ref())
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

    pub fn write(&mut self, value: &Value) -> io::Result<()> {
        match value {
            &Value::U64(ref v)    => self.write_header(MajorType::UINT, *v),
            &Value::I64(ref v)    => self.write_header(MajorType::NINT, *v as u64),
            &Value::Bytes(ref v)  => self.write_bs(&v),
            &Value::Array(ref v)  => self.write_array(&v),
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
            &ObjectKey::Integer(ref v) => self.write_header(MajorType::UINT, *v)
        }
    }
}

trait Read {
    fn next(&mut self) -> Option<u8>;
    fn peek(&self) -> Option<u8>;
    fn discard(&mut self);
    fn read(&mut self, len: usize) -> Vec<u8>;
    fn read_into(&mut self, buf: &mut [u8]) -> usize;
}
impl Read for Vec<u8> {
    fn next(&mut self) -> Option<u8> {
        if self.len() > 0 { Some(self.remove(0)) } else { None }
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
    reader: R,
    scope:  Vec<()>
}
impl<R> Decoder<R> where R: Read {
    pub fn new(reader: R) -> Self { Decoder { reader: reader, scope: vec![] } }

    fn consume(&mut self) { self.reader.discard() }

    pub fn peek_type(&mut self) -> Option<MajorType> {
        self.reader.peek().map(MajorType::from_byte)
    }

    fn u8(&mut self) -> Option<u64> { self.reader.next().map(|b| { b as u64 } ) }
    fn u16(&mut self) -> Option<u64> {
        let b1 = self.u8()?;
        let b2 = self.u8()?;
        Some(b1 << 8 | b2)
    }
    fn u32(&mut self) -> Option<u64> {
        let b1 = self.u8()?;
        let b2 = self.u8()?;
        let b3 = self.u8()?;
        let b4 = self.u8()?;
        Some(b1 << 24 | b2 << 16 | b3 << 8 | b4)
    }
    fn u64(&mut self) -> Option<u64> {
        let b1 = self.u8()?;
        let b2 = self.u8()?;
        let b3 = self.u8()?;
        let b4 = self.u8()?;
        let b5 = self.u8()?;
        let b6 = self.u8()?;
        let b7 = self.u8()?;
        let b8 = self.u8()?;
        Some(b1 << 56 | b2 << 48 | b3 << 40 | b4 << 32 | b5 << 24 | b6 << 16 | b7 << 8 | b8)
    }

    fn get_minor(&mut self) -> Option<u8> {
        self.reader.peek().map(|b| { b & 0b0001_1111 } )
    }

    fn get_minor_type(&mut self) -> Option<u64> {
        let b = self.get_minor()?;
        match b & 0b0001_1111 {
            0x00...0x17 => { self.consume(); Some(b as u64) },
            0x18        => { self.consume(); self.u8() },
            0x19        => { self.consume(); self.u16() },
            0x1a        => { self.consume(); self.u32() },
            0x1b        => { self.consume(); self.u64() },
            0x1c...0x1e => None,
            0x1f        => None,
            _           => None
        }
    }

    fn key(&mut self) -> Option<ObjectKey> {
        let ty = self.peek_type()?;
        match ty {
            MajorType::UINT => { self.get_minor_type().map(ObjectKey::Integer) },
            _ => None,
        }
    }

    pub fn value(&mut self) -> Option<Value> {
        let ty = self.peek_type()?;
        match ty {
            MajorType::UINT  => { self.get_minor_type().map(Value::U64) },
            MajorType::NINT  => { self.get_minor_type().map(|v| Value::I64(v as i64)) },
            MajorType::BYTES => {
                let len = self.get_minor_type()?;
                let buf = self.reader.read(len as usize);
                if len as usize != buf.len() { None } else { Some(Value::Bytes(buf) ) }
            },
            MajorType::TEXT  => { unimplemented!() }
            MajorType::ARRAY => {
                let maybe_len = self.get_minor_type();
                match maybe_len {
                    None      => {
                        match self.get_minor()? {
                            0x1F => Some(Value::ArrayStart),
                            _    => None
                        }
                    },
                    Some(len) => {
                        let mut array = vec![];
                        for _ in 0..len { array.push(self.value()?); }
                        Some(Value::Array(array))
                    }
                }
            },
            MajorType::MAP => {
                let maybe_len = self.get_minor_type();
                match maybe_len {
                    None      => { unimplemented!() /* test for an Indefinite array */ },
                    Some(len) => {
                        let mut map = BTreeMap::new();
                        for _ in 0..len {
                            let k = self.key()?;
                            let v = self.value()?;
                            map.insert(k, v);
                        }
                        Some(Value::Object(map))
                    }
                }
            },
            MajorType::TAG => {
                let tag = self.get_minor_type()?;
                let obj = self.value()?;
                Some(Value::Tag(tag, Box::new(obj)))
            },
            MajorType::T7 => {
                match self.get_minor_type() {
                    Some(0x1F) => Some(Value::Break),
                    _          => Some(Value::Null),
                }
            }
        }
    }
}

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
/*
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
