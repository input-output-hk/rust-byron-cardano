use cardano::block::HeaderHash;

pub const HASH_SIZE : usize = 32;

pub type BlockHash = [u8;HASH_SIZE];
pub type PackHash = [u8;HASH_SIZE];

pub fn header_to_blockhash(header_hash: &HeaderHash) -> BlockHash {
    let mut bh = [0u8;HASH_SIZE];
    bh[0..HASH_SIZE].clone_from_slice(header_hash.as_ref());
    bh
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum StorageFileType {
    Pack,
    Index,
    Blob,
    Tag,
    RefPack,
    Epoch,
}
