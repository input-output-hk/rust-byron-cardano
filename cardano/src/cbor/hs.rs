//! the CBOR util and compatible with the haskell usage...

pub mod util {
    //! CBor util and other stuff

    use cbor_event::{self, Len, de::{RawCbor}, Bytes};
    use crc32::{crc32};

    pub fn encode_with_crc32_<T, W>(t: &T, s: cbor_event::se::Serializer<W>) -> cbor_event::Result<cbor_event::se::Serializer<W>>
        where T: cbor_event::Serialize
            , W: ::std::io::Write + Sized
    {
        let bytes = t.serialize(cbor_event::se::Serializer::new_vec())?.finalize();
        let crc32 = crc32(&bytes);
        s.write_array(Len::Len(2))?
            .write_tag(24)?.write_bytes(&bytes)?
            .write_unsigned_integer(crc32 as u64)
    }
    pub fn raw_with_crc32<'a, 'b>(raw: &'b mut RawCbor<'a>) -> cbor_event::Result<Bytes<'a>> {
        let len = raw.array()?;
        assert!(len == Len::Len(2));

        let tag = raw.tag()?;
        if tag != 24 {
            return Err(cbor_event::Error::CustomError(format!("Invalid Tag: {} but expected 24", tag)));
        }
        let bytes = raw.bytes()?;

        let crc = raw.unsigned_integer()?;

        let found_crc = crc32(&bytes);

        if crc != found_crc as u64 {
            return Err(cbor_event::Error::CustomError(format!("Invalid CRC32: 0x{:x} but expected 0x{:x}", crc, found_crc)));
        }

        Ok(bytes)
    }

    pub fn decode_sum_type(raw: &mut RawCbor) -> cbor_event::Result<u64> {
        let len = raw.array()?;
        if len != cbor_event::Len::Len(2) {
            return Err(cbor_event::Error::CustomError(
                format!("Expected sum type but got array of {:?} elements", len)));
        }
        Ok(raw.unsigned_integer()?)
    }

    #[cfg(test)]
    #[cfg(feature = "with-bench")]
    mod bench {
        use super::*;
        use cbor_event::{self, de::RawCbor, se::{Serialize, Serializer}};

        #[cfg(feature = "with-bench")]
        use test;

        const CBOR : &'static [u8] = &[0x82, 0xd8, 0x18, 0x53, 0x52, 0x73, 0x6f, 0x6d, 0x65, 0x20, 0x72, 0x61, 0x6e, 0x64, 0x6f, 0x6d, 0x20, 0x73, 0x74, 0x72, 0x69, 0x6e, 0x67, 0x1a, 0x71, 0xad, 0x58, 0x36];

        const BYTES : &'static [u8] = b"some bytes";

        #[bench]
        fn encode_crc32_with_cbor_event(b: &mut test::Bencher) {
            b.iter(|| {
                let _ = encode_with_crc32_(&Test(BYTES), Serializer::new_vec()).unwrap();
            })
        }

        #[bench]
        fn decode_crc32_with_cbor_event(b: &mut test::Bencher) {
            b.iter(|| {
                let mut raw = RawCbor::from(CBOR);
                let bytes = raw_with_crc32(&mut raw).unwrap();
            })
        }

        struct Test(&'static [u8]);
        impl Serialize for Test {
            fn serialize<W>(&self, serializer: Serializer<W>) -> cbor_event::Result<Serializer<W>>
                where W: ::std::io::Write
            {
                serializer.write_bytes(self.0)
            }
        }
    }
}
