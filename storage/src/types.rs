pub const HASH_SIZE : usize = 32;

pub type BlockHash = [u8;HASH_SIZE];
pub type PackHash = [u8;HASH_SIZE];

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum StorageFileType {
    Pack, Index, Blob, Tag
}