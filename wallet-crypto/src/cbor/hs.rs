//! the CBOR util and compatible with the haskell usage...

#[cfg(test)]
use cbor::spec::{CborValue, encode_to_cbor, decode_from_cbor};
#[cfg(test)]
use raw_cbor::de;

#[cfg(test)]
pub fn encode_decode<T: CborValue+Eq+de::Deserialize>(t: &T) -> bool {
    let buf = encode_to_cbor(t).unwrap();

    print!("what where encoded: ");
    buf.iter().for_each(|b| {if *b<0x10 {print!("0{:x}", b);} else { print!("{:x}", b);}});
    println!("");

    let mut raw = de::RawCbor::from(&buf);
    let v = de::Deserialize::deserialize(&mut raw).expect("Should have decoded the CBOR");

    t == &v
}

pub mod util {
    //! CBor util and other stuff

    use cbor;
    use raw_cbor::{self, Len, de::{RawCbor, Bytes}};
    use cbor::spec::{ExtendedResult};
    use crc32::{crc32};

    pub fn encode_with_crc32<T: cbor::CborValue>(t: &T) -> cbor::Value {
        let v = cbor::encode_to_cbor(t).unwrap();
        let crc32 = crc32(&v);
        cbor::Value::Array(
            vec![ cbor::Value::Tag(24, Box::new(cbor::Value::Bytes(cbor::Bytes::new(v))))
                , cbor::Value::U64(crc32 as u64)
                ]
        )
    }
    pub fn raw_with_crc32<'a, 'b>(raw: &'b mut RawCbor<'a>) -> raw_cbor::Result<Bytes<'a>> {
        let len = raw.array()?;
        assert!(len == Len::Len(2));

        let tag = raw.tag()?;
        if *tag != 24 {
            return Err(raw_cbor::Error::CustomError(format!("Invalid Tag: {} but expected 24", *tag)));
        }
        let bytes = raw.bytes()?;

        let crc = raw.unsigned_integer()?;

        let found_crc = crc32(&bytes);

        if *crc != found_crc as u64 {
            return Err(raw_cbor::Error::CustomError(format!("Invalid CRC32: 0x{:x} but expected 0x{:x}", *crc, found_crc)));
        }

        Ok(bytes)
    }
    pub fn decode_with_crc32<T: cbor::CborValue>(value: cbor::Value) -> cbor::Result<T> {
        value.array().and_then(|array| {
            let (array, tag) : (Vec<cbor::Value>, cbor::Value) = cbor::array_decode_elem(array, 0)
                .embed("tagged element for crc32")?;
            let (array, crc) : (Vec<cbor::Value>, u32) = cbor::array_decode_elem(array, 0).embed("crc32 value")?;
            if array.len() != 0 {
                return cbor::Result::array(array, cbor::Error::UnparsedValues);
            }
            let bytes = tag.tag()
                .and_then(|(t, b)| {
                    if t != 24 {
                        cbor::Result::tag(t, b, cbor::Error::InvalidTag(t))
                    } else {
                        (*b).bytes()
                    }
                }).embed("while decoding the tagged bytes")?;
            let found_crc = crc32(bytes.as_ref());
            if crc != found_crc {
                cbor::Result::u64(crc as u64, cbor::Error::InvalidValue(Box::new(cbor::Value::U64(found_crc as u64))))
                    .embed("invalid CRC32")
            } else {
                cbor::decode_from_cbor(bytes.as_ref())
            }
        }).embed("crc32 encoded CborValue")
    }

    pub fn decode_sum_type(input: &[u8]) -> Option<(u8, &[u8])> {
        if input.len() > 2 && input[0] == 0x82 && input[1] < 23 {
            Some((input[1], &input[2..]))
        } else {
            None
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use cbor;

        #[test]
        fn crc32() {
            let bytes : Vec<u8> = b"some random string".iter().cloned().collect();
            let v = cbor::Bytes::new(bytes);
            let dest = encode_with_crc32(&v);
            let r : cbor::Bytes = decode_with_crc32(dest).unwrap();
            assert_eq!(v, r);
        }
    }

    #[cfg(test)]
    #[cfg(feature = "with-bench")]
    mod bench {
        use super::*;
        use cbor;
        use raw_cbor::de::RawCbor;

        #[cfg(feature = "with-bench")]
        use test;

        const CBOR : &'static [u8] = &[0x82, 0xd8, 0x18, 0x53, 0x52, 0x73, 0x6f, 0x6d, 0x65, 0x20, 0x72, 0x61, 0x6e, 0x64, 0x6f, 0x6d, 0x20, 0x73, 0x74, 0x72, 0x69, 0x6e, 0x67, 0x1a, 0x71, 0xad, 0x58, 0x36];

        #[bench]
        fn decode_crc32_with_raw_cbor(b: &mut test::Bencher) {
            b.iter(|| {
                let mut raw = RawCbor::from(CBOR);
                let bytes = raw_with_crc32(&mut raw).unwrap();
            })
        }

        #[bench]
        fn decode_crc32_with_value_cbor(b: &mut test::Bencher) {
            b.iter(|| {
                let value: cbor::Value = cbor::decode_from_cbor(CBOR).unwrap();
                let bytes : cbor::Bytes = decode_with_crc32(value).unwrap();
            })
        }
    }
}
