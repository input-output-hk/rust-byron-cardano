#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub enum MajorType {
    UINT,
    NINT,
    BYTES,
    TEXT,
    ARRAY,
    MAP,
    TAG,
    T7
}

impl MajorType {
    // serialize a major type in its highest bit form
    fn to_byte(self) -> u8 {
        use self::MajorType::*;
        match self {
            UINT  => 0b0000_0000,
            NINT  => 0b0010_0000,
            BYTES => 0b0100_0000,
            TEXT  => 0b0110_0000,
            ARRAY => 0b1000_0000,
            MAP   => 0b1010_0000,
            TAG   => 0b1100_0000,
            T7    => 0b1110_0000
        }
    }

    fn from_byte(byte: u8) -> Self {
        use self::MajorType::*;
        match byte & 0b1110_0000 {
            0b0000_0000 => UINT,
            0b0010_0000 => NINT,
            0b0100_0000 => BYTES,
            0b0110_0000 => TEXT,
            0b1000_0000 => ARRAY,
            0b1010_0000 => MAP,
            0b1100_0000 => TAG,
            0b1110_0000 => T7,
            _           => panic!("the impossible happened!")
        }
    }
}

const MAX_INLINE_ENCODING : u8 = 23;
const CBOR_PAYLOAD_LENGTH_U8 : u8 = 24;
const CBOR_PAYLOAD_LENGTH_U16 : u8 = 25;
const CBOR_PAYLOAD_LENGTH_U32 : u8 = 26;
const CBOR_PAYLOAD_LENGTH_U64 : u8 = 27;

// internal mobule to encode the address metadata in cbor to
// hash them.
//
pub mod encode {
    use cbor::*;

    pub fn cbor_header(ty: MajorType, r: u8) -> u8 {
        ty.to_byte() | r & 0x1f
    }

    pub fn cbor_uint_small(v: u8, buf: &mut Vec<u8>) {
        assert!(v <= MAX_INLINE_ENCODING);
        buf.push(cbor_header(MajorType::UINT, v));
    }

    pub fn cbor_u8(v: u8, buf: &mut Vec<u8>) {
        buf.push(cbor_header(MajorType::UINT, CBOR_PAYLOAD_LENGTH_U8));
        buf.push(v);
    }

    /// convenient macro to get the given bytes of the given value
    ///
    /// does all the job: Big Endian, bit shift and convertion
    macro_rules! byte_slice {
        ($value:ident, $shift:expr) => ({
            ($value >> $shift) as u8
        });
    }

    pub fn write_u8(v: u8, buf: &mut Vec<u8>) {
        write_header_u8(MajorType::UINT, v, buf);
    }
    pub fn write_u16(v: u16, buf: &mut Vec<u8>) {
        write_header_u16(MajorType::UINT, v, buf);
    }
    pub fn write_u32(v: u32, buf: &mut Vec<u8>) {
        write_header_u32(MajorType::UINT, v, buf);
    }
    pub fn write_u64(v: u64, buf: &mut Vec<u8>) {
        write_header_u64(MajorType::UINT, v, buf);
    }
    pub fn write_header_u8(ty: MajorType, v: u8, buf: &mut Vec<u8>) {
        buf.push(cbor_header(ty, CBOR_PAYLOAD_LENGTH_U8));
        buf.push(v);
    }
    pub fn write_header_u16(ty: MajorType, v: u16, buf: &mut Vec<u8>) {
        buf.push(cbor_header(ty, CBOR_PAYLOAD_LENGTH_U16));
        buf.push(byte_slice!(v, 8));
        buf.push(byte_slice!(v, 0));
    }
    pub fn write_header_u32(ty: MajorType, v: u32, buf: &mut Vec<u8>) {
        buf.push(cbor_header(ty, CBOR_PAYLOAD_LENGTH_U32));
        buf.push(byte_slice!(v, 24));
        buf.push(byte_slice!(v, 16));
        buf.push(byte_slice!(v,  8));
        buf.push(byte_slice!(v,  0));
    }
    pub fn write_header_u64(ty: MajorType, v: u64, buf: &mut Vec<u8>) {
        buf.push(cbor_header(ty, CBOR_PAYLOAD_LENGTH_U64));
        buf.push(byte_slice!(v, 56));
        buf.push(byte_slice!(v, 48));
        buf.push(byte_slice!(v, 40));
        buf.push(byte_slice!(v, 32));
        buf.push(byte_slice!(v, 24));
        buf.push(byte_slice!(v, 16));
        buf.push(byte_slice!(v,  8));
        buf.push(byte_slice!(v,  0));
    }

    pub fn write_length_encoding(ty: MajorType, nb_elems: u64, buf: &mut Vec<u8>) {
        if nb_elems <= (MAX_INLINE_ENCODING as u64) {
            buf.push(cbor_header(ty, nb_elems as u8));
        } else {
            if nb_elems < 0x100 {
                write_header_u8(ty, nb_elems as u8, buf);
            } else if nb_elems < 0x10000 {
                write_header_u16(ty, nb_elems as u16, buf);
            } else if nb_elems < 0x100000000 {
                write_header_u32(ty, nb_elems as u32, buf);
            } else {
                write_header_u64(ty, nb_elems as u64, buf);
            }
        }
    }

    pub fn cbor_uint(uint: u64, buf: &mut Vec<u8>) {
        write_length_encoding(MajorType::UINT, uint, buf);
    }

    pub fn cbor_tag(tag: u64, buf: &mut Vec<u8>) {
        write_length_encoding(MajorType::TAG, tag, buf);
    }

    pub fn cbor_bs(bs: &[u8], buf: &mut Vec<u8>) {
        write_length_encoding(MajorType::BYTES, bs.len() as u64, buf);
        buf.extend_from_slice(bs)
    }

    pub fn cbor_array_start(nb_elems: usize, buf: &mut Vec<u8>) {
        write_length_encoding(MajorType::ARRAY, nb_elems as u64, buf);
    }
    pub fn cbor_map_start(nb_elems: usize, buf: &mut Vec<u8>) {
        write_length_encoding(MajorType::MAP, nb_elems as u64, buf);
    }

}


// internal mobule to encode the address metadata in cbor to
// hash them.
//
pub mod decode {
    use cbor::*;
    use std::result;

    #[derive(Debug, PartialEq, Eq)]
    pub enum Error {
        NotEnough,
        WrongMajorType(MajorType, MajorType),
        InvalidPayloadLength(u8, u8),
        InvalidLength(usize, usize),
        InlineIntegerTooLarge,
        Custom(&'static str)
    }

    pub type Result<T> = result::Result<T, Error>;

    pub struct Decoder { buf: Vec<u8> }
    impl Decoder {
        pub fn new() -> Self { Decoder { buf: vec![] } }

        pub fn extend(&mut self, more: &[u8]) {
            self.buf.extend_from_slice(more)
        }

        fn drop(&mut self) -> Result<u8> {
            if self.buf.len() > 0 {
                Ok(self.buf.remove(0))
            } else {
                Err(Error::NotEnough)
            }
        }

        fn get_header(&mut self) -> Result<(MajorType, u8)> {
            let mt = MajorType::from_byte(self.buf[0]);
            let b = self.drop()?;
            Ok((mt, b & 0b001_1111))
        }
        fn header(&mut self, mt: MajorType) -> Result<u8> {
            let (found_mt, b) = self.get_header()?;
            if found_mt == mt {
                Ok(b)
            } else {
                Err(Error::WrongMajorType(found_mt, mt))
            }
        }

        pub fn uint_small(&mut self) -> Result<u8> {
            let b = self.header(MajorType::UINT)?;
            if b <= MAX_INLINE_ENCODING {
                self.drop();
                Ok(b)
            } else {
                Err(Error::InlineIntegerTooLarge)
            }
        }

        pub fn u8(&mut self) -> Result<u8> {
            let b = self.header(MajorType::UINT)?;
            if b == CBOR_PAYLOAD_LENGTH_U8 {
                self.drop()
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U8, b))
            }
        }
        pub fn u16(&mut self) -> Result<u16> {
            let b = self.header(MajorType::UINT)?;
            if b == CBOR_PAYLOAD_LENGTH_U16 {
                let h = self.drop()? as u16;
                let l = self.drop()? as u16;
                Ok(h << 8 | l)
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U16, b))
            }
        }
        pub fn u32(&mut self) -> Result<u32> {
            let b = self.header(MajorType::UINT)?;
            if b == CBOR_PAYLOAD_LENGTH_U32 {
                let x1 = self.drop()? as u32;
                let x2 = self.drop()? as u32;
                let x3 = self.drop()? as u32;
                let x4 = self.drop()? as u32;
                Ok(x1 << 24 | x2 << 16 | x3 << 8 | x4)
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U32, b))
            }
        }
        pub fn u64(&mut self) -> Result<u64> {
            let b = self.header(MajorType::UINT)?;
            if b == CBOR_PAYLOAD_LENGTH_U64 {
                let x1 = self.drop()? as u64;
                let x2 = self.drop()? as u64;
                let x3 = self.drop()? as u64;
                let x4 = self.drop()? as u64;
                let x5 = self.drop()? as u64;
                let x6 = self.drop()? as u64;
                let x7 = self.drop()? as u64;
                let x8 = self.drop()? as u64;
                Ok(x1 << 56 | x2 << 48 | x3 << 40 | x4 << 32 | x5 << 24 | x6 << 16 | x7 << 8 | x8)
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U64, b))
            }
        }

        pub fn length_header(&mut self) -> Result<(MajorType, usize)> {
            let (mt, x) = self.get_header()?;
            let b = x as u8;
            if x <= MAX_INLINE_ENCODING {
                Ok((mt, b as usize))
            } else if b == CBOR_PAYLOAD_LENGTH_U8 {
                let x1 = self.drop()? as usize;
                Ok((mt, x1))
            } else if b == CBOR_PAYLOAD_LENGTH_U16 {
                let x1 = self.drop()? as usize;
                let x2 = self.drop()? as usize;
                Ok((mt, x1 << 8 | x2))
            } else if b == CBOR_PAYLOAD_LENGTH_U32 {
                let x1 = self.drop()? as usize;
                let x2 = self.drop()? as usize;
                let x3 = self.drop()? as usize;
                let x4 = self.drop()? as usize;
                Ok((mt, x1 << 24 | x2 << 16 | x3 << 8 | x4))
            } else if b == CBOR_PAYLOAD_LENGTH_U64 {
                let x1 = self.drop()? as u64;
                let x2 = self.drop()? as u64;
                let x3 = self.drop()? as u64;
                let x4 = self.drop()? as u64;
                let x5 = self.drop()? as u64;
                let x6 = self.drop()? as u64;
                let x7 = self.drop()? as u64;
                let x8 = self.drop()? as u64;
                Ok((mt, (x1 << 56 | x2 << 48 | x3 << 40 | x4 << 32 | x5 << 24 | x6 << 16 | x7 << 8 | x8) as usize))
            } else {
                Err(Error::InvalidPayloadLength(CBOR_PAYLOAD_LENGTH_U64, b))
            }
        }

        fn length_header_type(&mut self, expected_mt: MajorType) -> Result<usize> {
            let (mt, l) = self.length_header()?;
            if mt == expected_mt {
                Ok(l)
            } else {
                Err(Error::WrongMajorType(expected_mt, mt))
            }
        }

        pub fn uint(&mut self) -> Result<u64> {
            let l = self.length_header_type(MajorType::UINT)?;
            Ok(l as u64)
        }

        pub fn tag(&mut self) -> Result<u64> {
            let l = self.length_header_type(MajorType::TAG)?;
            Ok(l as u64)
        }

        pub fn bs(&mut self) -> Result<Vec<u8>> {
            let l = self.length_header_type(MajorType::BYTES)?;
            let rem = self.buf.split_off(l);
            let r = self.buf.iter().cloned().collect();
            self.buf = rem;
            Ok(r)
        }

        pub fn array_start(&mut self) -> Result<usize> {
            self.length_header_type(MajorType::ARRAY)
        }
        pub fn map_start(&mut self) -> Result<usize> {
            self.length_header_type(MajorType::MAP)
        }
    }

}
