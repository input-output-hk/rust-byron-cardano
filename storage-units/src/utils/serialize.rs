// basic utility

pub type Offset = u64;
pub type Size = u32;

pub const OFF_SIZE: usize = 8;
pub const SIZE_SIZE: usize = 4;

pub fn offset_align4(p: Offset) -> Offset {
    let r = p % 4;
    if r == 0 {
        p
    } else {
        p.checked_add(4 - r).expect("offset too large")
    }
}

// write size to the mutable buffer in big endian
pub fn write_size(buf: &mut [u8], sz: Size) {
    buf[0] = (sz >> 24) as u8;
    buf[1] = (sz >> 16) as u8;
    buf[2] = (sz >> 8) as u8;
    buf[3] = sz as u8;
}

// read size from a buffer in big endian
pub fn read_size(buf: &[u8]) -> Size {
    ((buf[0] as Size) << 24) | ((buf[1] as Size) << 16) | ((buf[2] as Size) << 8) | (buf[3] as Size)
}

pub fn write_offset(buf: &mut [u8], sz: Offset) {
    buf[0] = (sz >> 56) as u8;
    buf[1] = (sz >> 48) as u8;
    buf[2] = (sz >> 40) as u8;
    buf[3] = (sz >> 32) as u8;
    buf[4] = (sz >> 24) as u8;
    buf[5] = (sz >> 16) as u8;
    buf[6] = (sz >> 8) as u8;
    buf[7] = sz as u8;
}

pub fn read_offset(buf: &[u8]) -> Offset {
    ((buf[0] as u64) << 56)
        | ((buf[1] as u64) << 48)
        | ((buf[2] as u64) << 40)
        | ((buf[3] as u64) << 32)
        | ((buf[4] as u64) << 24)
        | ((buf[5] as u64) << 16)
        | ((buf[6] as u64) << 8)
        | (buf[7] as u64)
}

pub mod io {
    use super::{Size, SIZE_SIZE};
    use std::io::{Error, ErrorKind, Read, Result, Write};

    #[inline]
    pub fn write_u8<W>(w: &mut W, byte: u8) -> Result<()>
    where
        W: Write,
    {
        w.write_all([byte].as_ref())
    }

    #[inline]
    pub fn write_u16<W>(w: &mut W, data: u16) -> Result<()>
    where
        W: Write,
    {
        write_u8(w, (data >> 8) as u8)?;
        write_u8(w, data as u8)
    }

    #[inline]
    pub fn write_u32<W>(w: &mut W, data: u32) -> Result<()>
    where
        W: Write,
    {
        write_u16(w, (data >> 16) as u16)?;
        write_u16(w, data as u16)
    }

    #[inline]
    pub fn write_u64<W>(w: &mut W, data: u64) -> Result<()>
    where
        W: Write,
    {
        write_u32(w, (data >> 32) as u32)?;
        write_u32(w, data as u32)
    }

    #[inline]
    pub fn read_u8<R>(r: &mut R) -> Result<u8>
    where
        R: Read,
    {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    #[inline]
    pub fn read_u16<R>(r: &mut R) -> Result<u16>
    where
        R: Read,
    {
        let b1 = (read_u8(r)? as u16) << 8;
        let b2 = read_u8(r)? as u16;
        Ok(b1 | b2)
    }
    #[inline]
    pub fn read_u32<R>(r: &mut R) -> Result<u32>
    where
        R: Read,
    {
        let b1 = (read_u16(r)? as u32) << 16;
        let b2 = read_u16(r)? as u32;
        Ok(b1 | b2)
    }
    #[inline]
    pub fn read_u64<R>(r: &mut R) -> Result<u64>
    where
        R: Read,
    {
        let b1 = (read_u32(r)? as u64) << 32;
        let b2 = read_u32(r)? as u64;
        Ok(b1 | b2)
    }

    /// Returns the length of the passed byte slice if its length is acceptable
    /// for serialization as a `Size` value.
    ///
    /// # Errors
    /// Returns I/O error of the kind `InvalidInput` if the length is too large
    /// to be represented.
    ///
    fn validate_len(input: &[u8]) -> Result<Size> {
        let len = input.len();
        if len <= Size::max_value() as usize {
            Ok(len as Size)
        } else {
            Err(Error::new(
                ErrorKind::InvalidInput,
                format!("value length {} is too large", len),
            ))
        }
    }

    /// Writes a `Size` value to the generic output.
    pub fn write_size<W>(w: &mut W, data: Size) -> Result<()>
    where
        W: Write,
    {
        let mut buf = [0; SIZE_SIZE];
        super::write_size(&mut buf, data);
        w.write_all(&buf)
    }

    /// Writes the sequence of bytes given in the slice parameter,
    /// prefixed by the length of the sequence and padded to the next
    /// 32-bit aligned offset.
    /// Returns the total size in bytes written.
    pub fn write_length_prefixed<W>(w: &mut W, data: &[u8]) -> Result<u64>
    where
        W: Write,
    {
        let len = validate_len(data)?;
        write_size(w, len)?;
        w.write_all(data)?;
        let pad = [0u8; SIZE_SIZE - 1];
        let pad_bytes = if len % 4 != 0 {
            let pad_sz = 4 - len % 4;
            w.write_all(&pad[0..pad_sz as usize])?;
            pad_sz
        } else {
            0
        };
        Ok(4 + len as u64 + pad_bytes as u64) // Overflow can't occur here
    }
}
