use utils::bitmap;

const FNV_PRIME: u64 = 0x100000001b3;
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;

// calculate FNV1, FNV1a
pub fn hash(content: &[u8]) -> (u64, u64) {
    let mut hash = FNV_OFFSET_BASIS;
    let mut hash2 = FNV_OFFSET_BASIS;
    for c in content {
        // FNV1
        hash = hash.wrapping_mul(FNV_PRIME);
        hash ^= *c as u64;

        // FNV1a
        hash2 = *c as u64;
        hash2 = hash2.wrapping_mul(FNV_PRIME);
    }
    (hash, hash2)
}

pub fn addr3(max: usize, content: &[u8]) -> (usize, usize, usize) {
    let (f1, f2) = hash(content);

    let v1 = f1 as usize % max;
    let v2 = f2 as usize % max;
    let v3 = ((f1 ^ f2) >> 32) as usize % max;
    (v1, v2, v3)
}

pub fn set(bitmap: &mut [u8], content: &[u8]) {
    let (v1, v2, v3) = addr3(bitmap.len() * 8, content);

    bitmap::set_bit_to(bitmap, v1, true);
    bitmap::set_bit_to(bitmap, v2, true);
    bitmap::set_bit_to(bitmap, v3, true);
}

pub fn is_set(bitmap: &[u8], content: &[u8]) -> bool {
    let (v1, v2, v3) = addr3(bitmap.len() * 8, content);
    bitmap::get_bit(bitmap, v1) && bitmap::get_bit(bitmap, v2) && bitmap::get_bit(bitmap, v3)
}
