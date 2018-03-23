//! the CBOR util and compatible with the haskell usage...

use cbor::spec::{MajorType, CborValue, encode_to_cbor};
use cbor::spec::decode;

pub fn dec_sumtype_start(decoder: &mut decode::Decoder) -> decode::Result<(u64, usize)> {
    let l = decoder.array_start()?;
    let t = decoder.uint()?;
    Ok((t, l - 1))
}

#[cfg(test)]
pub fn encode_decode<T: CborValue+FromCBOR+Eq>(t: &T) -> bool {
    let buf = encode_to_cbor(t).unwrap();

    print!("what where encoded: ");
    buf.iter().for_each(|b| {if *b<0x10 {print!("0{:x}", b);} else { print!("{:x}", b);}});
    println!("");

    let mut dec = decode::Decoder::new();
    dec.extend(buf.as_ref());
    let v = FromCBOR::decode(&mut dec).unwrap();
    t == &v
}

pub trait FromCBOR : Sized {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self>;
}
impl FromCBOR for u8 {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> { Ok(decoder.uint()? as u8) }
}
impl FromCBOR for u16 {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> { Ok(decoder.uint()? as u16) }
}
impl FromCBOR for u32 {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> { Ok(decoder.uint()? as u32) }
}
impl FromCBOR for u64 {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> { decoder.uint() }
}
impl FromCBOR for Vec<u8> {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> {
        decoder.bs()
    }
}
impl <A: FromCBOR + Sized, B: FromCBOR + Sized> FromCBOR for (A, B) {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> {
        let l = decoder.array_start()?;
        let x = A::decode(decoder)?;
        let y = B::decode(decoder)?;
        Ok((x,y))
    }
}
impl <A: FromCBOR + Sized, B: FromCBOR + Sized, C: FromCBOR + Sized> FromCBOR for (A, B, C) {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> {
        let l = decoder.array_start()?;
        let x = A::decode(decoder)?;
        let y = B::decode(decoder)?;
        let z = C::decode(decoder)?;
        Ok((x,y,z))
    }
}

pub fn deserialize<T: FromCBOR>(buf: &[u8]) -> decode::Result<T> {
    let mut dec = decode::Decoder::new();
    dec.extend(buf);
    T::decode(&mut dec)
}

pub mod util {
    //! CBor util and other stuff

    use cbor;
    use cbor::decode;
    use cbor::decode::{Decoder};
    use cbor::hs::{FromCBOR, deserialize};
    use crc32::{crc32};

    pub fn encode_with_crc32<T: cbor::CborValue>(t: &T) -> cbor::Value {
        let v = cbor::encode_to_cbor(t).unwrap();
        let crc32 = crc32(&v);
        cbor::Value::Array(
            vec![ cbor::Value::Tag(24, Box::new(cbor::Value::Bytes(v)))
                , cbor::Value::U64(crc32 as u64)
                ]
        )
    }
    pub fn decode_with_crc32<T: FromCBOR>(decoder: &mut Decoder) -> decode::Result<T> {
        let len = decoder.array_start()?;
        assert!(len == 2);
        let tag = decoder.tag()?;
        assert!(tag == 24);
        let buf = decoder.bs()?;
        let crc = decoder.u32()?;

        assert!(crc == crc32(&buf));

        deserialize(&buf)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use cbor::spec::decode::{Decoder};
        use cbor;

        #[test]
        fn crc32() {
            let bytes : Vec<u8> = b"some random string".iter().cloned().collect();
            let dest = cbor::encode_to_cbor(&encode_with_crc32::<Vec<u8>>(&bytes)).unwrap();
            let mut decoder = Decoder::new();
            decoder.extend(dest.as_ref());
            let r : Vec<u8> = decode_with_crc32::<Vec<u8>>(&mut decoder).unwrap();
            assert_eq!(bytes, r);
        }
    }
}
