use crate::{Address, AddressReadable};
use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Serialize, Serializer},
};
use std::fmt;

impl Serialize for Address {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if serializer.is_human_readable() {
            AddressReadable::from_address(self).serialize(serializer)
        } else {
            serializer.serialize_bytes(&self.to_bytes())
        }
    }
}

impl Serialize for AddressReadable {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_string())
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            deserializer
                .deserialize_str(AddressReadableVisitor)
                .map(|address_readable| address_readable.to_address())
        } else {
            deserializer.deserialize_bytes(AddressVisitor)
        }
    }
}

impl<'de> Deserialize<'de> for AddressReadable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(AddressReadableVisitor)
    }
}

struct AddressVisitor;
struct AddressReadableVisitor;

impl<'de> Visitor<'de> for AddressVisitor {
    type Value = Address;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting an Address",)
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        use chain_core::mempack::{ReadBuf, Readable};
        let mut buf = ReadBuf::from(v);
        match Self::Value::read(&mut buf) {
            Err(err) => Err(E::custom(err)),
            Ok(address) => Ok(address),
        }
    }
}

impl<'de> Visitor<'de> for AddressReadableVisitor {
    type Value = AddressReadable;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Expecting an Address",)
    }

    fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        use std::str::FromStr;
        match Self::Value::from_str(v) {
            Err(err) => Err(E::custom(err)),
            Ok(address) => Ok(address),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{Address, AddressReadable};

    use bincode;
    use serde_json;

    quickcheck! {
        fn address_encode_decode_bincode(address: Address) -> bool {
            let encoded = bincode::serialize(&address).unwrap();
            let decoded : Address = bincode::deserialize(&encoded).unwrap();

            decoded == address
        }
        fn address_encode_decode_json(address: Address) -> bool {
            let encoded = serde_json::to_string(&address).unwrap();
            let decoded : Address= serde_json::from_str(&encoded).unwrap();

            decoded == address
        }

        fn address_readable_encode_decode_bincode(address: AddressReadable) -> bool {
            let encoded = bincode::serialize(&address).unwrap();
            let decoded: AddressReadable = bincode::deserialize(&encoded).unwrap();

            decoded == address
        }
        fn address_readable_encode_decode_json(address: AddressReadable) -> bool {
            let encoded = serde_json::to_string(&address).unwrap();
            let decoded:AddressReadable = serde_json::from_str(&encoded).unwrap();

            decoded == address
        }
    }
}
