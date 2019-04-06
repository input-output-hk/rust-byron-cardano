//! module to add serde serializer and deserializer to the
//! different types defined here
//!

use crate::{
    bech32::{Bech32 as _, Error as Bech32Error},
    AsymmetricKey, PublicKey, PublicKeyError, SecretKey, SecretKeyError, Signature, SignatureError,
    VerificationAlgorithm,
};
use serde::{
    de::{Deserialize, Deserializer, Error, Visitor},
    ser::{Serialize, Serializer},
};
use std::fmt;

impl<A: AsymmetricKey> Serialize for SecretKey<A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_bech32_str())
        } else {
            serializer.serialize_bytes(self.0.as_ref())
        }
    }
}
impl<A: AsymmetricKey> Serialize for PublicKey<A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_bech32_str())
        } else {
            serializer.serialize_bytes(self.as_ref())
        }
    }
}

impl<T, A: VerificationAlgorithm> Serialize for Signature<T, A> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_bech32_str())
        } else {
            serializer.serialize_bytes(self.as_ref())
        }
    }
}

impl<'de, A> Deserialize<'de> for SecretKey<A>
where
    A: AsymmetricKey,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secret_key_visitor = SecretKeyVisitor::new();
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(secret_key_visitor)
        } else {
            deserializer.deserialize_bytes(secret_key_visitor)
        }
    }
}

impl<'de, A> Deserialize<'de> for PublicKey<A>
where
    A: AsymmetricKey,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let public_key_visitor = PublicKeyVisitor::new();
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(public_key_visitor)
        } else {
            deserializer.deserialize_bytes(public_key_visitor)
        }
    }
}

impl<'de, T, A> Deserialize<'de> for Signature<T, A>
where
    A: VerificationAlgorithm,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let signature_visitor = SignatureVisitor::new();
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(signature_visitor)
        } else {
            deserializer.deserialize_bytes(signature_visitor)
        }
    }
}

struct SecretKeyVisitor<A: AsymmetricKey> {
    _marker: std::marker::PhantomData<A>,
}
struct PublicKeyVisitor<A: AsymmetricKey> {
    _marker: std::marker::PhantomData<A>,
}
struct SignatureVisitor<T, A: VerificationAlgorithm> {
    _marker_1: std::marker::PhantomData<T>,
    _marker_2: std::marker::PhantomData<A>,
}
impl<A: AsymmetricKey> SecretKeyVisitor<A> {
    #[inline]
    fn new() -> Self {
        SecretKeyVisitor {
            _marker: std::marker::PhantomData,
        }
    }
}
impl<A: AsymmetricKey> PublicKeyVisitor<A> {
    #[inline]
    fn new() -> Self {
        PublicKeyVisitor {
            _marker: std::marker::PhantomData,
        }
    }
}
impl<T, A: VerificationAlgorithm> SignatureVisitor<T, A> {
    #[inline]
    fn new() -> Self {
        SignatureVisitor {
            _marker_1: std::marker::PhantomData,
            _marker_2: std::marker::PhantomData,
        }
    }
}

impl<'de, A> Visitor<'de> for SecretKeyVisitor<A>
where
    A: AsymmetricKey,
{
    type Value = SecretKey<A>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "Expecting a secret key for algorithm {}",
            A::SECRET_BECH32_HRP
        )
    }

    fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match Self::Value::try_from_bech32_str(&v) {
            Err(Bech32Error::DataInvalid(err)) => Err(E::custom(format!("Invalid data: {}", err))),
            Err(Bech32Error::HrpInvalid { expected, actual }) => Err(E::custom(format!(
                "Invalid prefix: expected {} but was {}",
                expected, actual
            ))),
            Err(Bech32Error::Bech32Malformed(err)) => {
                Err(E::custom(format!("Invalid bech32: {}", err)))
            }
            Ok(key) => Ok(key),
        }
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match Self::Value::from_binary(v) {
            Err(SecretKeyError::SizeInvalid) => Err(E::custom(format!(
                "Invalid size (expected: {}bytes)",
                A::SECRET_KEY_SIZE
            ))),
            Err(SecretKeyError::StructureInvalid) => Err(E::custom("Invalid structure")),
            Ok(key) => Ok(key),
        }
    }
}

impl<'de, A> Visitor<'de> for PublicKeyVisitor<A>
where
    A: AsymmetricKey,
{
    type Value = PublicKey<A>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "Expecting a public key for algorithm {}",
            A::PUBLIC_BECH32_HRP
        )
    }

    fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match Self::Value::try_from_bech32_str(&v) {
            Err(Bech32Error::DataInvalid(err)) => Err(E::custom(format!("Invalid data: {}", err))),
            Err(Bech32Error::HrpInvalid { expected, actual }) => Err(E::custom(format!(
                "Invalid prefix: expected {} but was {}",
                expected, actual
            ))),
            Err(Bech32Error::Bech32Malformed(err)) => {
                Err(E::custom(format!("Invalid bech32: {}", err)))
            }
            Ok(key) => Ok(key),
        }
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match Self::Value::from_binary(v) {
            Err(PublicKeyError::SizeInvalid) => Err(E::custom(format!(
                "Invalid size (expected: {}bytes)",
                A::PUBLIC_KEY_SIZE
            ))),
            Err(PublicKeyError::StructureInvalid) => Err(E::custom("Invalid structure")),
            Ok(key) => Ok(key),
        }
    }
}

impl<'de, T, A> Visitor<'de> for SignatureVisitor<T, A>
where
    A: VerificationAlgorithm,
{
    type Value = Signature<T, A>;

    fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(
            fmt,
            "Expecting a signature for algorithm {}",
            A::SIGNATURE_BECH32_HRP
        )
    }

    fn visit_str<'a, E>(self, v: &'a str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match Self::Value::try_from_bech32_str(&v) {
            Err(Bech32Error::DataInvalid(err)) => Err(E::custom(format!("Invalid data: {}", err))),
            Err(Bech32Error::HrpInvalid { expected, actual }) => Err(E::custom(format!(
                "Invalid prefix: expected {} but was {}",
                expected, actual
            ))),
            Err(Bech32Error::Bech32Malformed(err)) => {
                Err(E::custom(format!("Invalid bech32: {}", err)))
            }
            Ok(key) => Ok(key),
        }
    }

    fn visit_bytes<'a, E>(self, v: &'a [u8]) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match Self::Value::from_binary(v) {
            Err(SignatureError::SizeInvalid) => Err(E::custom(format!(
                "Invalid size (expected: {}bytes)",
                A::PUBLIC_KEY_SIZE
            ))),
            Err(SignatureError::StructureInvalid) => Err(E::custom("Invalid structure")),
            Ok(key) => Ok(key),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        algorithms::{Curve25519_2HashDH, Ed25519, Ed25519Bip32, Ed25519Extended, FakeMMM},
        hash::{Blake2b224, Blake2b256},
    };

    use bincode;
    use serde_json;

    quickcheck! {
        fn ed25519_secret_key_encode_decode_bincode(secret: SecretKey<Ed25519>) -> bool {
            let encoded = bincode::serialize(&secret).unwrap();
            let decoded : SecretKey<Ed25519> = bincode::deserialize(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }
        fn ed25519_secret_key_encode_decode_json(secret: SecretKey<Ed25519>) -> bool {
            let encoded = serde_json::to_string(&secret).unwrap();
            let decoded : SecretKey<Ed25519> = serde_json::from_str(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }

        fn ed25519bip32_secret_key_encode_decode_bincode(secret: SecretKey<Ed25519Bip32>) -> bool {
            let encoded = bincode::serialize(&secret).unwrap();
            let decoded : SecretKey<Ed25519Bip32> = bincode::deserialize(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }
        fn ed25519bip32_secret_key_encode_decode_json(secret: SecretKey<Ed25519Bip32>) -> bool {
            let encoded = serde_json::to_string(&secret).unwrap();
            let decoded : SecretKey<Ed25519Bip32> = serde_json::from_str(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }

        fn ed25519extended_secret_key_encode_decode_bincode(secret: SecretKey<Ed25519Extended>) -> bool {
            let encoded = bincode::serialize(&secret).unwrap();
            let decoded : SecretKey<Ed25519Extended> = bincode::deserialize(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }
        fn ed25519extended_secret_key_encode_decode_json(secret: SecretKey<Ed25519Extended>) -> bool {
            let encoded = serde_json::to_string(&secret).unwrap();
            let decoded : SecretKey<Ed25519Extended> = serde_json::from_str(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }

        fn fakemmm_secret_key_encode_decode_bincode(secret: SecretKey<FakeMMM>) -> bool {
            let encoded = bincode::serialize(&secret).unwrap();
            let decoded : SecretKey<FakeMMM> = bincode::deserialize(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }
        fn fakemmm_secret_key_encode_decode_json(secret: SecretKey<FakeMMM>) -> bool {
            let encoded = serde_json::to_string(&secret).unwrap();
            let decoded : SecretKey<FakeMMM> = serde_json::from_str(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }

        fn curve25519_2hashdh_secret_key_encode_decode_bincode(secret: SecretKey<Curve25519_2HashDH>) -> bool {
            let encoded = bincode::serialize(&secret).unwrap();
            let decoded : SecretKey<Curve25519_2HashDH> = bincode::deserialize(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }
        fn curve25519_2hashdh_secret_key_encode_decode_json(secret: SecretKey<Curve25519_2HashDH>) -> bool {
            let encoded = serde_json::to_string(&secret).unwrap();
            let decoded : SecretKey<Curve25519_2HashDH> = serde_json::from_str(&encoded).unwrap();

            secret.0.as_ref() == decoded.0.as_ref()
        }

        fn ed25519_public_key_encode_decode_bincode(public: PublicKey<Ed25519>) -> bool {
            let encoded = bincode::serialize(&public).unwrap();
            let decoded : PublicKey<Ed25519> = bincode::deserialize(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }
        fn ed25519_public_key_encode_decode_json(public: PublicKey<Ed25519>) -> bool {
            let encoded = serde_json::to_string(&public).unwrap();
            let decoded : PublicKey<Ed25519> = serde_json::from_str(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }

        fn ed25519bip32_public_key_encode_decode_bincode(public: PublicKey<Ed25519Bip32>) -> bool {
            let encoded = bincode::serialize(&public).unwrap();
            let decoded : PublicKey<Ed25519Bip32> = bincode::deserialize(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }
        fn ed25519bip32_public_key_encode_decode_json(public: PublicKey<Ed25519Bip32>) -> bool {
            let encoded = serde_json::to_string(&public).unwrap();
            let decoded : PublicKey<Ed25519Bip32> = serde_json::from_str(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }

        fn ed25519extended_public_key_encode_decode_bincode(public: PublicKey<Ed25519Extended>) -> bool {
            let encoded = bincode::serialize(&public).unwrap();
            let decoded : PublicKey<Ed25519Extended> = bincode::deserialize(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }
        fn ed25519extended_public_key_encode_decode_json(public: PublicKey<Ed25519Extended>) -> bool {
            let encoded = serde_json::to_string(&public).unwrap();
            let decoded : PublicKey<Ed25519Extended> = serde_json::from_str(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }

        fn fakemmm_public_key_encode_decode_bincode(public: PublicKey<FakeMMM>) -> bool {
            let encoded = bincode::serialize(&public).unwrap();
            let decoded : PublicKey<FakeMMM> = bincode::deserialize(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }
        fn fakemmm_public_key_encode_decode_json(public: PublicKey<FakeMMM>) -> bool {
            let encoded = serde_json::to_string(&public).unwrap();
            let decoded : PublicKey<FakeMMM> = serde_json::from_str(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }

        fn curve25519_2hashdh_public_key_encode_decode_bincode(public: PublicKey<Curve25519_2HashDH>) -> bool {
            let encoded = bincode::serialize(&public).unwrap();
            let decoded : PublicKey<Curve25519_2HashDH> = bincode::deserialize(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }
        fn curve25519_2hashdh_public_key_encode_decode_json(public: PublicKey<Curve25519_2HashDH>) -> bool {
            let encoded = serde_json::to_string(&public).unwrap();
            let decoded : PublicKey<Curve25519_2HashDH> = serde_json::from_str(&encoded).unwrap();

            public.0.as_ref() == decoded.0.as_ref()
        }

        fn ed25519_signature_key_encode_decode_bincode(signature: Signature<Vec<u8>, Ed25519>) -> bool {
            let encoded = bincode::serialize(&signature).unwrap();
            let decoded : Signature<Vec<u8>, Ed25519> = bincode::deserialize(&encoded).unwrap();

            signature.as_ref() == decoded.as_ref()
        }
        fn ed25519_signature_key_encode_decode_json(signature: Signature<Vec<u8>, Ed25519>) -> bool {
            let encoded = serde_json::to_string(&signature).unwrap();
            let decoded : Signature<Vec<u8>, Ed25519> = serde_json::from_str(&encoded).unwrap();

            signature.as_ref() == decoded.as_ref()
        }

        fn ed25519bip32_signature_key_encode_decode_bincode(signature: Signature<Vec<u8>, Ed25519Bip32>) -> bool {
            let encoded = bincode::serialize(&signature).unwrap();
            let decoded : Signature<Vec<u8>, Ed25519Bip32> = bincode::deserialize(&encoded).unwrap();

            signature.as_ref() == decoded.as_ref()
        }
        fn ed25519bip32_signature_key_encode_decode_json(signature: Signature<Vec<u8>, Ed25519Bip32>) -> bool {
            let encoded = serde_json::to_string(&signature).unwrap();
            let decoded : Signature<Vec<u8>, Ed25519Bip32> = serde_json::from_str(&encoded).unwrap();

            signature.as_ref() == decoded.as_ref()
        }

        fn ed25519extended_signature_key_encode_decode_bincode(signature: Signature<Vec<u8>, Ed25519Extended>) -> bool {
            let encoded = bincode::serialize(&signature).unwrap();
            let decoded : Signature<Vec<u8>, Ed25519Extended> = bincode::deserialize(&encoded).unwrap();

            signature.as_ref() == decoded.as_ref()
        }
        fn ed25519extended_signature_key_encode_decode_json(signature: Signature<Vec<u8>, Ed25519Extended>) -> bool {
            let encoded = serde_json::to_string(&signature).unwrap();
            let decoded : Signature<Vec<u8>, Ed25519Extended> = serde_json::from_str(&encoded).unwrap();

            signature.as_ref() == decoded.as_ref()
        }

        fn fakemmm_signature_key_encode_decode_bincode(signature: Signature<Vec<u8>, FakeMMM>) -> bool {
            let encoded = bincode::serialize(&signature).unwrap();
            let decoded : Signature<Vec<u8>, FakeMMM> = bincode::deserialize(&encoded).unwrap();

            signature.as_ref() == decoded.as_ref()
        }
        fn fakemmm_signature_key_encode_decode_json(signature: Signature<Vec<u8>, FakeMMM>) -> bool {
            let encoded = serde_json::to_string(&signature).unwrap();
            let decoded : Signature<Vec<u8>, FakeMMM> = serde_json::from_str(&encoded).unwrap();

            signature.as_ref() == decoded.as_ref()
        }

        fn hash_blake2b_224_encode_decode_bincode(hash: Blake2b224) -> bool {
            let encoded = bincode::serialize(&hash).unwrap();
            let decoded : Blake2b224 = bincode::deserialize(&encoded).unwrap();

            hash.as_ref() == decoded.as_ref()
        }
        fn hash_blake2b_224_encode_decode_json(hash: Blake2b224) -> bool {
            let encoded = serde_json::to_string(&hash).unwrap();
            let decoded : Blake2b224 = serde_json::from_str(&encoded).unwrap();

            hash.as_ref() == decoded.as_ref()
        }

        fn hash_blake2b_256_encode_decode_bincode(hash: Blake2b256) -> bool {
            let encoded = bincode::serialize(&hash).unwrap();
            let decoded : Blake2b256 = bincode::deserialize(&encoded).unwrap();

            hash.as_ref() == decoded.as_ref()
        }
        fn hash_blake2b_256_encode_decode_json(hash: Blake2b256) -> bool {
            let encoded = serde_json::to_string(&hash).unwrap();
            let decoded : Blake2b256 = serde_json::from_str(&encoded).unwrap();

            hash.as_ref() == decoded.as_ref()
        }
    }
}
