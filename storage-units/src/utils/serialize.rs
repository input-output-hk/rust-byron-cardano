// basic utility

pub type Offset = u64;
pub type Size = u32;

pub const OFF_SIZE : usize = 8;
pub const SIZE_SIZE : usize = 4;

pub fn offset_align4(p: Offset) -> Offset {
    if (p % 4) == 0 {
        p
    } else {
        p + (4 - (p % 4))
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
    ((buf[0] as Size) << 24)
        | ((buf[1] as Size) << 16)
        | ((buf[2] as Size) << 8)
        | (buf[3] as Size)
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
        | ((buf[7] as u64))
}

pub mod utils {
    use std::io::{Result, Write, Read};

    #[inline]
    pub fn write_u8<W>(w: &mut W, byte: u8) -> Result<()>
        where W: Write
    {
        w.write_all([byte].as_ref())
    }

    #[inline]
    pub fn write_u16<W>(w: &mut W, data: u16) -> Result<()>
        where W: Write
    {
        write_u8(w, (data >> 8) as u8)?;
        write_u8(w,  data       as u8)
    }

    #[inline]
    pub fn write_u32<W>(w: &mut W, data: u32) -> Result<()>
        where W: Write
    {
        write_u16(w, (data >> 16) as u16)?;
        write_u16(w,  data        as u16)
    }

    #[inline]
    pub fn write_u64<W>(w: &mut W, data: u64) -> Result<()>
        where W: Write
    {
        write_u32(w, (data >> 32) as u32)?;
        write_u32(w,  data        as u32)
    }


    #[inline]
    pub fn read_u8<R>(r: &mut R) -> Result<u8>
        where R: Read
    {
        let mut buf = [0u8;1];
        r.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    #[inline]
    pub fn read_u16<R>(r: &mut R) -> Result<u16>
        where R: Read
    {
        let b1 = (read_u8(r)? as u16) << 8;
        let b2 =  read_u8(r)? as u16;
        Ok(b1 | b2)
    }
    #[inline]
    pub fn read_u32<R>(r: &mut R) -> Result<u32>
        where R: Read
    {
        let b1 = (read_u16(r)? as u32) << 16;
        let b2 =  read_u16(r)? as u32;
        Ok(b1 | b2)
    }
    #[inline]
    pub fn read_u64<R>(r: &mut R) -> Result<u64>
        where R: Read
    {
        let b1 = (read_u32(r)? as u64) << 32;
        let b2 =  read_u32(r)? as u64;
        Ok(b1 | b2)
    }
}
