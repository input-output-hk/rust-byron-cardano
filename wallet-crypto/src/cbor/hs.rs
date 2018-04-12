//! the CBOR util and compatible with the haskell usage...

#[cfg(test)]
use cbor::spec::{CborValue, encode_to_cbor, decode_from_cbor};

#[cfg(test)]
pub fn encode_decode<T: CborValue+Eq>(t: &T) -> bool {
    let buf = encode_to_cbor(t).unwrap();

    print!("what where encoded: ");
    buf.iter().for_each(|b| {if *b<0x10 {print!("0{:x}", b);} else { print!("{:x}", b);}});
    println!("");

    let v = decode_from_cbor(buf.as_ref()).expect("Should have decoded the CBOR");

    t == &v
}

pub mod util {
    //! CBor util and other stuff

    use cbor;
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
}
