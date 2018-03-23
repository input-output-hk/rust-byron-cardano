//! CBor as specified by the RFC

use std::collections::BTreeMap;
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

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    U64(u64),
    I64(i64),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(BTreeMap<ObjectKey, Value>),
    Tag(u64, Box<Value>),
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

    pub fn write(&mut self, value: &Value) -> io::Result<()> {
        match value {
            &Value::U64(ref v)    => self.write_header(MajorType::UINT, *v),
            &Value::I64(ref v)    => self.write_header(MajorType::NINT, *v as u64),
            &Value::Bytes(ref v)  => self.write_bs(&v),
            &Value::Array(ref v)  => self.write_array(&v),
            &Value::Object(ref v) => self.write_object(&v),
            &Value::Tag(ref t, ref v) => {
                self.write_header(MajorType::TAG, *t)?;
                self.write(v.as_ref())
            },
            &Value::Null         => Ok(())
        }
    }
    pub fn write_key(&mut self, key: &ObjectKey) -> io::Result<()> {
        match key {
            &ObjectKey::Integer(ref v) => self.write_header(MajorType::UINT, *v)
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

// internal mobule to encode the address metadata in cbor to
// hash them.
//
pub mod decode {
    use super::*;
    use std::result;

    #[derive(Debug, PartialEq, Eq)]
    pub enum Error {
        NotEnough,
        WrongMajorType(MajorType, MajorType),
        InvalidPayloadLength(u8, u8),
        InvalidLength(usize, usize),
        InlineIntegerTooLarge,
        Custom(&'static str)
    }

    pub type Result<T> = result::Result<T, Error>;

    pub struct Decoder { buf: Vec<u8> }
    impl Decoder {
        pub fn new() -> Self { Decoder { buf: vec![] } }

        pub fn extend(&mut self, more: &[u8]) {
            self.buf.extend_from_slice(more)
        }

        fn drop(&mut self) -> Result<u8> {
            if self.buf.len() > 0 {
                Ok(self.buf.remove(0))
            } else {
                Err(Error::NotEnough)
            }
        }

        fn get_header(&mut self) -> Result<(MajorType, u8)> {
            let mt = MajorType::from_byte(self.buf[0]);
            let b = self.drop()?;
            Ok((mt, b & 0b001_1111))
        }
        fn header(&mut self, mt: MajorType) -> Result<u8> {
            let (found_mt, b) = self.get_header()?;
            if found_mt == mt {
                Ok(b)
            } else {
                Err(Error::WrongMajorType(found_mt, mt))
            }
        }

        pub fn uint_small(&mut self) -> Result<u8> {
            let b = self.header(MajorType::UINT)?;
            if b <= MAX_INLINE_ENCODING {
                self.drop();
                Ok(b)
            } else {
                Err(Error::InlineIntegerTooLarge)
            }
        }

        pub fn u8(&mut self) -> Result<u8> {
            let b = self.header(MajorType::UINT)?;
            if b == CBOR_PAYLOAD_LENGTH_U8 {
                self.drop()
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U8, b))
            }
        }
        pub fn u16(&mut self) -> Result<u16> {
            let b = self.header(MajorType::UINT)?;
            if b == CBOR_PAYLOAD_LENGTH_U16 {
                let h = self.drop()? as u16;
                let l = self.drop()? as u16;
                Ok(h << 8 | l)
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U16, b))
            }
        }
        pub fn u32(&mut self) -> Result<u32> {
            let b = self.header(MajorType::UINT)?;
            if b == CBOR_PAYLOAD_LENGTH_U32 {
                let x1 = self.drop()? as u32;
                let x2 = self.drop()? as u32;
                let x3 = self.drop()? as u32;
                let x4 = self.drop()? as u32;
                Ok(x1 << 24 | x2 << 16 | x3 << 8 | x4)
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U32, b))
            }
        }
        pub fn u64(&mut self) -> Result<u64> {
            let b = self.header(MajorType::UINT)?;
            if b == CBOR_PAYLOAD_LENGTH_U64 {
                let x1 = self.drop()? as u64;
                let x2 = self.drop()? as u64;
                let x3 = self.drop()? as u64;
                let x4 = self.drop()? as u64;
                let x5 = self.drop()? as u64;
                let x6 = self.drop()? as u64;
                let x7 = self.drop()? as u64;
                let x8 = self.drop()? as u64;
                Ok(x1 << 56 | x2 << 48 | x3 << 40 | x4 << 32 | x5 << 24 | x6 << 16 | x7 << 8 | x8)
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U64, b))
            }
        }

        pub fn length_header(&mut self) -> Result<(MajorType, usize)> {
            let (mt, x) = self.get_header()?;
            let b = x as u8;
            if x <= MAX_INLINE_ENCODING {
                Ok((mt, b as usize))
            } else if b == CBOR_PAYLOAD_LENGTH_U8 {
                let x1 = self.drop()? as usize;
                Ok((mt, x1))
            } else if b == CBOR_PAYLOAD_LENGTH_U16 {
                let x1 = self.drop()? as usize;
                let x2 = self.drop()? as usize;
                Ok((mt, x1 << 8 | x2))
            } else if b == CBOR_PAYLOAD_LENGTH_U32 {
                let x1 = self.drop()? as usize;
                let x2 = self.drop()? as usize;
                let x3 = self.drop()? as usize;
                let x4 = self.drop()? as usize;
                Ok((mt, x1 << 24 | x2 << 16 | x3 << 8 | x4))
            } else if b == CBOR_PAYLOAD_LENGTH_U64 {
                let x1 = self.drop()? as u64;
                let x2 = self.drop()? as u64;
                let x3 = self.drop()? as u64;
                let x4 = self.drop()? as u64;
                let x5 = self.drop()? as u64;
                let x6 = self.drop()? as u64;
                let x7 = self.drop()? as u64;
                let x8 = self.drop()? as u64;
                Ok((mt, (x1 << 56 | x2 << 48 | x3 << 40 | x4 << 32 | x5 << 24 | x6 << 16 | x7 << 8 | x8) as usize))
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U64, b))
            }
        }

        fn length_header_type(&mut self, expected_mt: MajorType) -> Result<usize> {
            let (mt, l) = self.length_header()?;
            if mt == expected_mt {
                Ok(l)
            } else {
                Err(Error::WrongMajorType(expected_mt, mt))
            }
        }

        pub fn uint(&mut self) -> Result<u64> {
            let l = self.length_header_type(MajorType::UINT)?;
            Ok(l as u64)
        }

        pub fn tag(&mut self) -> Result<u64> {
            let l = self.length_header_type(MajorType::TAG)?;
            Ok(l as u64)
        }

        pub fn bs(&mut self) -> Result<Vec<u8>> {
            let l = self.length_header_type(MajorType::BYTES)?;
            let rem = self.buf.split_off(l);
            let r = self.buf.iter().cloned().collect();
            self.buf = rem;
            Ok(r)
        }

        pub fn array_start(&mut self) -> Result<usize> {
            self.length_header_type(MajorType::ARRAY)
        }
        pub fn map_start(&mut self) -> Result<usize> {
            self.length_header_type(MajorType::MAP)
        }
    }

}
