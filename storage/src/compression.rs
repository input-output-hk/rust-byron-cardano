use flate2::Compression;
use flate2::write::DeflateEncoder;
use flate2::write::DeflateDecoder;
use std::io::{Error, Write};

pub fn decompress_conditional(input: &[u8]) -> Vec<u8> {
    if super::USE_COMPRESSION {
        let mut writer = Vec::new();
        let mut deflater = DeflateDecoder::new(writer);
        deflater.write_all(&input[..]).unwrap();
        writer = deflater.finish().unwrap();
        writer
    } else {
        Vec::from(input)
    }
}

pub fn compress_conditional(input: &[u8]) -> Vec<u8> {
    if super::USE_COMPRESSION {
        let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
        e.write_all(input).unwrap();
        e.finish().unwrap()
    } else {
        Vec::from(input)
    }
}

pub fn compress_write<T: Write>(writer: &mut T, input: &[u8]) -> Result<(), Error> {
    if super::USE_COMPRESSION {
        let mut e = DeflateEncoder::new(Vec::new(), Compression::best());
        e.write_all(input)?;
        let compressed_block = e.finish()?;
        writer.write_all(&compressed_block[..])
    } else {
        writer.write_all(input)
    }
}
