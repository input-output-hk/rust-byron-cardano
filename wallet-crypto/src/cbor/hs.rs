//! the CBOR util and compatible with the haskell usage...

use cbor::spec::{MajorType, CborValue, encode_to_cbor, decode_from_cbor};

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
    use crc32::{crc32};
    use std::borrow::{Borrow};

    pub fn encode_with_crc32<T: cbor::CborValue>(t: &T) -> cbor::Value {
        let v = cbor::encode_to_cbor(t).unwrap();
        let crc32 = crc32(&v);
        cbor::Value::Array(
            vec![ cbor::Value::Tag(24, Box::new(cbor::Value::Bytes(v)))
                , cbor::Value::U64(crc32 as u64)
                ]
        )
    }
    pub fn decode_with_crc32<T: cbor::CborValue>(value: &cbor::Value) -> Option<T> {
        match value {
            &cbor::Value::Array(ref array) => {
                let bs = match cbor::CborValue::decode(array.get(0)?)? {
                    cbor::Value::Tag(24, ref c) => {
                        match c.borrow() {
                            &cbor::Value::Bytes(ref bytes) => bytes.clone(),
                            _ => return None,
                        }
                    },
                    _ => return None,
                };
                let crc : u32 = cbor::CborValue::decode(array.get(1)?)?;
                assert!(crc == crc32(&bs));
                cbor::decode_from_cbor(bs.as_ref())
            },
            _ => None
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn crc32() {
            let bytes : Vec<u8> = b"some random string".iter().cloned().collect();
            let dest = encode_with_crc32::<Vec<u8>>(&bytes);
            let r : Vec<u8> = decode_with_crc32(&dest).unwrap();
            assert_eq!(bytes, r);
        }
    }
}
