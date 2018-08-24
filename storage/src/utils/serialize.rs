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