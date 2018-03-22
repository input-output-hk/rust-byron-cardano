//! the CBOR util and compatible with the haskell usage...

use cbor::spec::{MajorType};
use cbor::spec::encode;
use cbor::spec::decode;

pub fn sumtype_start(tag: u64, nb_values: usize, buf: &mut Vec<u8>) -> () {
    encode::array_start(nb_values + 1, buf);
    // tag value from 0
    encode::uint(tag, buf);
}

pub fn dec_sumtype_start(decoder: &mut decode::Decoder) -> decode::Result<(u64, usize)> {
    let l = decoder.array_start()?;
    let t = decoder.uint()?;
    Ok((t, l - 1))
}

#[cfg(test)]
pub fn encode_decode<T: ToCBOR+FromCBOR+Eq>(t: &T) -> bool {
    let mut buf = vec![];
    t.encode(&mut buf);

    let mut dec = decode::Decoder::new();
    dec.extend(&buf);
    let v = T::decode(&mut dec).unwrap();
    t == &v
}

// helper trait to write CBOR encoding
pub trait ToCBOR {
    fn encode(&self, &mut Vec<u8>);
}
pub trait FromCBOR : Sized {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self>;
}
impl ToCBOR for Vec<u8> {
    fn encode(&self, buf: &mut Vec<u8>) {
        encode::bs(self, buf)
    }
}
impl FromCBOR for Vec<u8> {
    fn decode(decoder: &mut decode::Decoder) -> decode::Result<Self> {
        decoder.bs()
    }
}
impl<T: ToCBOR> ToCBOR for Option<T> {
    fn encode(&self, buf: &mut Vec<u8>) {
        match self {
            &None => sumtype_start(0, 0, buf),
            &Some(ref t) => {
                // TODO ? sumtype_start(1, 1, buf);
                t.encode(buf)
            }
        }
    }
}
impl <'a, 'b, A: ToCBOR, B: ToCBOR> ToCBOR for (&'a A, &'b B) {
    fn encode(&self, buf: &mut Vec<u8>) {
        encode::array_start(2, buf);
        self.0.encode(buf);
        self.1.encode(buf);
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
impl <'a, 'b, 'c, A: ToCBOR, B: ToCBOR, C: ToCBOR> ToCBOR for (&'a A, &'b B, &'c C) {
    fn encode(&self, buf: &mut Vec<u8>) {
        encode::write_length_encoding(MajorType::ARRAY, 3, buf);
        self.0.encode(buf);
        self.1.encode(buf);
        self.2.encode(buf);
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

pub fn serialize<T: ToCBOR>(t: &T) -> Vec<u8> {
    let mut buf = vec![];
    t.encode(&mut buf);
    buf
}
pub fn deserialize<T: FromCBOR>(buf: &[u8]) -> decode::Result<T> {
    let mut dec = decode::Decoder::new();
    dec.extend(buf);
    T::decode(&mut dec)
}

pub mod util {
    //! CBor util and other stuff

    use cbor::encode;
    use cbor::decode;
    use cbor::decode::{Decoder};
    use cbor::hs::{ToCBOR, FromCBOR, serialize, deserialize};
    use crc32::{crc32};

    pub fn encode_with_crc32<T: ToCBOR>(t: &T, buf: &mut Vec<u8>) {
        let v = serialize(t);

        encode::array_start(2, buf);
        encode::tag(24, buf);
        encode::bs(&v, buf);

        encode::write_u32(crc32(&v), buf);
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

        #[test]
        fn crc32() {
            let bytes : Vec<u8> = b"some random string".iter().cloned().collect();
            let mut dest = vec![];
            encode_with_crc32::<Vec<u8>>(&bytes, &mut dest);
            let mut decoder = Decoder::new();
            decoder.extend(&dest);
            let r : Vec<u8> = decode_with_crc32::<Vec<u8>>(&mut decoder).unwrap();
            assert_eq!(bytes, r);
        }
    }
}
