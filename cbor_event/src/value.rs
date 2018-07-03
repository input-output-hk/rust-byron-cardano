//! CBOR Value object representation
//!
//! While it is handy to be able to construct into the intermediate value
//! type it is also not recommended to use it as an intermediate type
//! before deserialising concrete type:
//!
//! - it is slow and bloated;
//! - it takes a lot dynamic memory and may not be compatible with the targeted environment;
//!
//! This is why all the objects here are marked as deprecated

use types::{Type, Special};
use result::Result;
use error::Error;
use len::Len;
use de::*;
use se::*;

use std::{collections::{BTreeMap}, io::Write};

/// CBOR Object key, represents the possible supported values for
/// a CBOR key in a CBOR Map.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectKey {
    Integer(u64),
    Bytes(Vec<u8>),
    Text(String),
}
impl ObjectKey {
    /// convert the given `ObjectKey` into a CBOR [`Value`](./struct.Value.html)
    pub fn value(self) -> Value {
        match self {
            ObjectKey::Integer(v) => Value::U64(v),
            ObjectKey::Bytes(v) => Value::Bytes(v),
            ObjectKey::Text(v) => Value::Text(v),
        }
    }
}
impl Serialize for ObjectKey {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        match self {
            ObjectKey::Integer(ref v) => serializer.write_unsigned_integer(*v),
            ObjectKey::Bytes(ref v) => serializer.write_bytes(v),
            ObjectKey::Text(ref v) => serializer.write_text(v),
        }
    }
}
impl Deserialize for ObjectKey {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        match raw.cbor_type()? {
            Type::UnsignedInteger => Ok(ObjectKey::Integer(raw.unsigned_integer()?)),
            Type::Bytes           => Ok(ObjectKey::Bytes(Vec::from(raw.bytes()?.as_ref()))),
            Type::Text            => Ok(ObjectKey::Text(raw.text()?)),
            t                     => Err(Error::CustomError(format!("Type `{:?}' is not a support type for CBOR Map's key", t)))
        }
    }
}

/// All possible CBOR supported values.
///
/// We advise not to use these objects as an intermediary representation before
/// retrieving custom types as it is a slow and not memory efficient way to do
/// so. However it is handy for debugging or reverse a given protocol.
///
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Value {
    U64(u64),
    I64(i64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Value>),
    IArray(Vec<Value>),
    Object(BTreeMap<ObjectKey, Value>),
    IObject(BTreeMap<ObjectKey, Value>),
    Tag(u64, Box<Value>),
    Special(Special)
}

impl Serialize for Value {
    fn serialize<W: Write+Sized>(&self, serializer: Serializer<W>) -> Result<Serializer<W>> {
        match self {
            Value::U64(ref v) => serializer.write_unsigned_integer(*v),
            Value::I64(ref v) => serializer.write_negative_integer(*v),
            Value::Bytes(ref v) => serializer.write_bytes(v),
            Value::Text(ref v) => serializer.write_text(v),
            Value::Array(ref v) => {
                let mut serializer = serializer.write_array(Len::Len(v.len() as u64))?;
                for element in v {
                    serializer = serializer.serialize(element)?;
                }
                Ok(serializer)
            },
            Value::IArray(ref v) => {
                let mut serializer = serializer.write_array(Len::Indefinite)?;
                for element in v {
                    serializer = serializer.serialize(element)?;
                }
                serializer.write_special(Special::Break)
            },
            Value::Object(ref v) => {
                let mut serializer = serializer.write_map(Len::Len(v.len() as u64))?;
                for element in v {
                    serializer = serializer.serialize(element.0)?
                                           .serialize(element.1)?;
                }
                Ok(serializer)
            },
            Value::IObject(ref v) => {
                let mut serializer = serializer.write_map(Len::Indefinite)?;
                for element in v {
                    serializer = serializer.serialize(element.0)?
                                           .serialize(element.1)?;
                }
                serializer.write_special(Special::Break)
            },
            Value::Tag(ref tag, ref v) => {
                serializer.write_tag(*tag)?.serialize(v.as_ref())
            },
            Value::Special(ref v) => serializer.write_special(*v)
        }
    }
}
impl Deserialize for Value {
    fn deserialize<'a>(raw: &mut RawCbor<'a>) -> Result<Self> {
        match raw.cbor_type()? {
            Type::UnsignedInteger => Ok(Value::U64(raw.unsigned_integer()?)),
            Type::NegativeInteger => Ok(Value::I64(raw.negative_integer()?)),
            Type::Bytes           => Ok(Value::Bytes(Vec::from(raw.bytes()?.as_ref()))),
            Type::Text            => Ok(Value::Text(raw.text()?)),
            Type::Array           => {
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
                        Ok(Value::IArray(vec))
                    },
                    Len::Len(len) => {
                        for _ in 0..len {
                            vec.push(Deserialize::deserialize(raw)?);
                        }
                        Ok(Value::Array(vec))
                    }
                }
            },
            Type::Map          => {
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
                        Ok(Value::IObject(vec))
                    },
                    Len::Len(len) => {
                        for _ in 0..len {
                            let k = Deserialize::deserialize(raw)?;
                            let v = Deserialize::deserialize(raw)?;
                            vec.insert(k, v);
                        }
                        Ok(Value::Object(vec))
                    }
                }
            },
            Type::Tag             => {
                let tag = raw.tag()?;
                Ok(Value::Tag(tag, Box::new(Deserialize::deserialize(raw)?)))
            },
            Type::Special         => Ok(Value::Special(raw.special()?)),
        }
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use super::super::{test_encode_decode};

    #[test]
    fn u64() {
        assert!(test_encode_decode(&Value::U64(0)).unwrap());
        assert!(test_encode_decode(&Value::U64(23)).unwrap());
        assert!(test_encode_decode(&Value::U64(0xff)).unwrap());
        assert!(test_encode_decode(&Value::U64(0x100)).unwrap());
        assert!(test_encode_decode(&Value::U64(0xffff)).unwrap());
        assert!(test_encode_decode(&Value::U64(0x10000)).unwrap());
        assert!(test_encode_decode(&Value::U64(0xffffffff)).unwrap());
        assert!(test_encode_decode(&Value::U64(0x100000000)).unwrap());
        assert!(test_encode_decode(&Value::U64(0xffffffffffffffff)).unwrap());
    }

    #[test]
    fn i64() {
        assert!(test_encode_decode(&Value::I64(0)).unwrap());
        assert!(test_encode_decode(&Value::I64(23)).unwrap());
        assert!(test_encode_decode(&Value::I64(-99)).unwrap());
        assert!(test_encode_decode(&Value::I64(99999)).unwrap());
        assert!(test_encode_decode(&Value::I64(-9999999)).unwrap());
        assert!(test_encode_decode(&Value::I64(-283749237289)).unwrap());
        assert!(test_encode_decode(&Value::I64(93892929229)).unwrap());
    }

    #[test]
    fn bytes() {
        assert!(test_encode_decode(&Value::Bytes(vec![])).unwrap());
        assert!(test_encode_decode(&Value::Bytes(vec![0;23])).unwrap());
        assert!(test_encode_decode(&Value::Bytes(vec![0;24])).unwrap());
        assert!(test_encode_decode(&Value::Bytes(vec![0;256])).unwrap());
        assert!(test_encode_decode(&Value::Bytes(vec![0;10293])).unwrap());
        assert!(test_encode_decode(&Value::Bytes(vec![0;99999000])).unwrap());
    }

    #[test]
    fn text() {
        assert!(test_encode_decode(&Value::Text("".to_owned())).unwrap());
        assert!(test_encode_decode(&Value::Text("hellow world".to_owned())).unwrap());
        assert!(test_encode_decode(&Value::Text("some sentence, some sentence... some sentence...some sentence, some sentence... some sentence...".to_owned())).unwrap());
    }

    #[test]
    fn array() {
        assert!(test_encode_decode(&Value::Array(vec![])).unwrap());
        assert!(test_encode_decode(&Value::Array(vec![Value::U64(0), Value::Text("some text".to_owned())])).unwrap());
    }

    #[test]
    fn iarray() {
        assert!(test_encode_decode(&Value::IArray(vec![])).unwrap());
        assert!(test_encode_decode(&Value::IArray(vec![Value::U64(0), Value::Text("some text".to_owned())])).unwrap());
    }

    #[test]
    fn tag() {
        assert!(test_encode_decode(&Value::Tag(23, Box::new(Value::U64(0)))).unwrap());
        assert!(test_encode_decode(&Value::Tag(24, Box::new(Value::Bytes(vec![0;32])))).unwrap());
        assert!(test_encode_decode(&Value::Tag(0x1ff, Box::new(Value::Bytes(vec![0;624])))).unwrap());
    }
}
