use crate::certificate::SignatureRaw;
use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Error as _, Serialize, Serializer},
};
use std::fmt;

const SIGNATURE_RAW_HRP: &'static str = "";

impl Serialize for SignatureRaw {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            use bech32::{Bech32, ToBase32};

            let string = Bech32::new(SIGNATURE_RAW_HRP.to_string(), self.0.to_base32())
                .map_err(S::Error::custom)?
                .to_string();

            serializer.serialize_str(&string)
        } else {
            serializer.serialize_bytes(self.0.as_ref())
        }
    }
}

impl<'de> Deserialize<'de> for SignatureRaw {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = BytesVisitor {
            hrp: SIGNATURE_RAW_HRP,
        };
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(visitor).map(SignatureRaw)
        } else {
            deserializer.deserialize_bytes(visitor).map(SignatureRaw)
        }
    }
}

/// helper for the generic serialization.
///
/// If encoding the data as a string: uses bech32;
///
/// If encoding the data as bytes, encode raw bytes
///
struct BytesVisitor {
    hrp: &'static str,
}

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "Expecting bytes or bech32 encoded bytes (with hrp: {})",
            self.hrp,
        )
    }

    fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        use bech32::{Bech32, FromBase32};
        let bech32: Bech32 = v.parse().map_err(E::custom)?;

        if bech32.hrp() != self.hrp {
            return Err(Error::custom(format!(
                "Invalid HRP ({}), expected: {}",
                bech32.hrp(),
                self.hrp
            )));
        }
        Vec::<u8>::from_base32(bech32.data()).map_err(E::custom)
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v.to_owned())
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(v)
    }
}
