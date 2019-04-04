//! Abstraction of either boundary or normal blocks
//!
//! The main types are `Header` and `Block`
use std::ops::{Deref, DerefMut};
use std::{
    fmt,
    io::{BufRead, Cursor, Write},
};

use super::super::cbor::hs::util::decode_sum_type;
use super::super::config::ProtocolMagic;
use super::boundary;
use super::date::BlockDate;
use super::normal;
use super::types::{BlockVersion, ChainDifficulty, HeaderHash};
use crate::tx::TxAux;
use cbor_event::{self, de::Deserialize, de::Deserializer, se::Serializer};
use chain_core;

#[derive(Debug, Clone)]
pub struct RawBlockHeaderMultiple(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct RawBlockHeader(pub Vec<u8>);

#[derive(Debug, Clone)]
pub struct RawBlock(pub Vec<u8>);

impl RawBlockHeaderMultiple {
    pub fn from_dat(dat: Vec<u8>) -> Self {
        RawBlockHeaderMultiple(dat)
    }
    pub fn decode(&self) -> cbor_event::Result<Vec<BlockHeader>> {
        let mut de = Deserializer::from(Cursor::new(&self.0));
        de.deserialize_complete()
    }
}
impl RawBlockHeader {
    pub fn from_dat(dat: Vec<u8>) -> Self {
        RawBlockHeader(dat)
    }
    pub fn decode(&self) -> cbor_event::Result<BlockHeader> {
        let mut de = Deserializer::from(Cursor::new(&self.0));
        de.deserialize_complete()
    }
    pub fn compute_hash(&self) -> HeaderHash {
        HeaderHash::new(&self.0)
    }
}
impl RawBlock {
    pub fn from_dat(dat: Vec<u8>) -> Self {
        RawBlock(dat)
    }
    pub fn decode(&self) -> cbor_event::Result<Block> {
        let mut de = Deserializer::from(Cursor::new(&self.0));
        de.deserialize_complete()
    }
    pub fn to_header(&self) -> cbor_event::Result<RawBlockHeader> {
        // TODO optimise if possible with the CBOR structure by skipping some prefix and some suffix ...
        let blk = self.decode()?;
        Ok(blk.header().to_raw())
    }
}

impl AsRef<[u8]> for RawBlockHeader {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl AsRef<[u8]> for RawBlock {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// Block Header of either a boundary header or a normal header
#[derive(Debug, Clone)]
pub enum BlockHeader {
    BoundaryBlockHeader(boundary::BlockHeader),
    MainBlockHeader(normal::BlockHeader),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChainLength(usize);

impl chain_core::property::ChainLength for ChainLength {
    fn next(&self) -> Self {
        ChainLength(self.0 + 1)
    }
}

impl chain_core::property::Header for BlockHeader {
    type Id = HeaderHash;
    type Date = BlockDate;
    type Version = BlockVersion;
    type ChainLength = ChainLength;

    fn id(&self) -> Self::Id {
        self.compute_hash()
    }

    fn parent_id(&self) -> Self::Id {
        match self {
            BlockHeader::BoundaryBlockHeader(ref header) => header.previous_header.clone(),
            BlockHeader::MainBlockHeader(ref header) => header.previous_header.clone(),
        }
    }

    fn date(&self) -> Self::Date {
        match self {
            BlockHeader::BoundaryBlockHeader(ref header) => header.consensus.epoch.into(),
            BlockHeader::MainBlockHeader(ref header) => header.consensus.slot_id.into(),
        }
    }

    fn version(&self) -> Self::Version {
        match self {
            BlockHeader::BoundaryBlockHeader(ref _header) => unimplemented!(),
            BlockHeader::MainBlockHeader(ref header) => header.extra_data.block_version,
        }
    }

    fn chain_length(&self) -> Self::ChainLength {
        unimplemented!()
    }
}

/// Accessor to the header block data.
///
/// `BlockHeaderView` is like `BlockHeader`, but it refers to data
/// inside a `Block` rather than owning them. It is a lightweight
/// accessor to block header data.
#[derive(Debug, Clone)]
pub enum BlockHeaderView<'a> {
    Boundary(&'a boundary::BlockHeader),
    Normal(&'a normal::BlockHeader),
}

impl<'a> From<BlockHeaderView<'a>> for BlockHeader {
    fn from(view: BlockHeaderView<'a>) -> BlockHeader {
        match view {
            BlockHeaderView::Boundary(hdr) => BlockHeader::BoundaryBlockHeader(hdr.clone()),
            BlockHeaderView::Normal(hdr) => BlockHeader::MainBlockHeader(hdr.clone()),
        }
    }
}

/// BlockHeaders is a vector of block headers, as produced by
/// MsgBlocks.
#[derive(Debug, Clone)]
pub struct BlockHeaders(pub Vec<BlockHeader>);

impl Deref for BlockHeaders {
    type Target = Vec<BlockHeader>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BlockHeaders {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'a> BlockHeaderView<'a> {
    /// Returns the hash of the previous block.
    pub fn previous_header(&self) -> HeaderHash {
        match self {
            BlockHeaderView::Boundary(hdr) => hdr.previous_header.clone(),
            BlockHeaderView::Normal(hdr) => hdr.previous_header.clone(),
        }
    }

    /// Returns the block date.
    pub fn blockdate(&self) -> BlockDate {
        match self {
            BlockHeaderView::Boundary(hdr) => BlockDate::Boundary(hdr.consensus.epoch),
            BlockHeaderView::Normal(hdr) => BlockDate::Normal(hdr.consensus.slot_id.clone()),
        }
    }

    /// Returns true if the block is the epoch's boundary block,
    /// otherwise returns false.
    pub fn is_boundary_block(&self) -> bool {
        match self {
            BlockHeaderView::Boundary(_) => true,
            BlockHeaderView::Normal(_) => false,
        }
    }

    fn to_cbor(&self) -> Vec<u8> {
        // the only reason this would fail is if there was no more memory
        // to allocate. This would be the users' last concern if it was
        // the case.
        cbor!(self).unwrap()
    }

    /// Serializes the block header into its raw data representation.
    pub fn to_raw(&self) -> RawBlockHeader {
        RawBlockHeader(self.to_cbor())
    }

    /// Computes the hash of the block header data.
    pub fn compute_hash(&self) -> HeaderHash {
        HeaderHash::new(&self.to_cbor())
    }

    pub fn difficulty(&self) -> ChainDifficulty {
        match self {
            BlockHeaderView::Boundary(h) => h.consensus.chain_difficulty,
            BlockHeaderView::Normal(h) => h.consensus.chain_difficulty,
        }
    }
}

impl BlockHeader {
    pub fn get_previous_header(&self) -> HeaderHash {
        match self {
            &BlockHeader::BoundaryBlockHeader(ref blo) => blo.previous_header.clone(),
            &BlockHeader::MainBlockHeader(ref blo) => blo.previous_header.clone(),
        }
    }

    pub fn get_blockdate(&self) -> BlockDate {
        match self {
            &BlockHeader::BoundaryBlockHeader(ref blo) => BlockDate::Boundary(blo.consensus.epoch),
            &BlockHeader::MainBlockHeader(ref blo) => {
                BlockDate::Normal(blo.consensus.slot_id.clone())
            }
        }
    }
    // TODO: TO REMOVE deprecated use get_blockdate
    pub fn get_slotid(&self) -> BlockDate {
        self.get_blockdate()
    }

    pub fn is_boundary_block(&self) -> bool {
        match self {
            &BlockHeader::BoundaryBlockHeader(_) => true,
            &BlockHeader::MainBlockHeader(_) => false,
        }
    }

    pub fn to_raw(&self) -> RawBlockHeader {
        // the only reason this would fail is if there was no more memory
        // to allocate. This would be the users' last concern if it was the
        // case
        RawBlockHeader(cbor!(self).unwrap())
    }

    pub fn compute_hash(&self) -> HeaderHash {
        // the only reason this would fail is if there was no more memory
        // to allocate. This would be the users' last concern if it was the
        // case
        let v = cbor!(self).unwrap();
        HeaderHash::new(&v[..])
    }

    pub fn difficulty(&self) -> ChainDifficulty {
        match self {
            BlockHeader::BoundaryBlockHeader(h) => h.consensus.chain_difficulty,
            BlockHeader::MainBlockHeader(h) => h.consensus.chain_difficulty,
        }
    }
}

impl fmt::Display for BlockHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &BlockHeader::BoundaryBlockHeader(ref mbh) => write!(f, "{}", mbh),
            &BlockHeader::MainBlockHeader(ref mbh) => write!(f, "{}", mbh),
        }
    }
}

impl chain_core::property::Serialize for BlockHeader {
    type Error = cbor_event::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        let mut serializer = cbor_event::se::Serializer::new(writer);
        serializer.serialize(self)?;
        serializer.finalize();
        Ok(())
    }
}

impl chain_core::property::Deserialize for BlockHeader {
    type Error = cbor_event::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Deserialize::deserialize(&mut Deserializer::from(reader))
    }
}

/// Block of either a boundary block or a normal block
#[derive(Debug, Clone)]
pub enum Block {
    BoundaryBlock(boundary::Block),
    MainBlock(normal::Block),
}
impl Block {
    pub fn is_boundary_block(&self) -> bool {
        match self {
            &Block::BoundaryBlock(_) => true,
            &Block::MainBlock(_) => false,
        }
    }

    pub fn header(&self) -> BlockHeaderView {
        match self {
            Block::BoundaryBlock(blk) => BlockHeaderView::Boundary(&blk.header),
            Block::MainBlock(blk) => BlockHeaderView::Normal(&blk.header),
        }
    }

    #[deprecated(note = "use header() instead")]
    pub fn get_header(&self) -> BlockHeader {
        match self {
            &Block::BoundaryBlock(ref blk) => BlockHeader::BoundaryBlockHeader(blk.header.clone()),
            &Block::MainBlock(ref blk) => BlockHeader::MainBlockHeader(blk.header.clone()),
        }
    }

    pub fn has_transactions(&self) -> bool {
        match self {
            &Block::BoundaryBlock(_) => false,
            &Block::MainBlock(ref blk) => blk.header.body_proof.tx.number > 0,
        }
    }

    pub fn get_transactions(&self) -> Option<normal::TxPayload> {
        match self {
            &Block::BoundaryBlock(_) => None,
            &Block::MainBlock(ref blk) => Some(blk.body.tx.clone()),
        }
    }

    pub fn get_protocol_magic(&self) -> ProtocolMagic {
        match self {
            &Block::BoundaryBlock(ref blk) => blk.header.protocol_magic,
            &Block::MainBlock(ref blk) => blk.header.protocol_magic,
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Block::BoundaryBlock(ref blk) => write!(f, "{}", blk),
            &Block::MainBlock(ref blk) => write!(f, "{}", blk),
        }
    }
}

impl chain_core::property::Block for Block {
    type Id = HeaderHash;
    type Date = BlockDate;
    type Version = BlockVersion;
    type ChainLength = ChainLength;

    fn id(&self) -> Self::Id {
        self.header().compute_hash()
    }

    fn parent_id(&self) -> Self::Id {
        match self {
            Block::MainBlock(ref block) => block.header.previous_header.clone(),
            Block::BoundaryBlock(ref block) => block.header.previous_header.clone(),
        }
    }

    fn date(&self) -> Self::Date {
        match self {
            Block::MainBlock(ref block) => block.header.consensus.slot_id.into(),
            Block::BoundaryBlock(ref block) => block.header.consensus.epoch.into(),
        }
    }

    fn version(&self) -> Self::Version {
        match self {
            Block::MainBlock(ref block) => block.header.extra_data.block_version,
            Block::BoundaryBlock(ref _block) => unimplemented!(),
        }
    }

    fn chain_length(&self) -> Self::ChainLength {
        unimplemented!()
    }
}

impl chain_core::property::HasHeader for Block {
    type Header = BlockHeader;

    fn header(&self) -> BlockHeader {
        self.header().into()
    }
}

impl chain_core::property::Serialize for Block {
    type Error = cbor_event::Error;

    fn serialize<W: std::io::Write>(&self, writer: W) -> Result<(), Self::Error> {
        let mut serializer = cbor_event::se::Serializer::new(writer);
        serializer.serialize(self)?;
        serializer.finalize();
        Ok(())
    }
}

impl chain_core::property::Deserialize for Block {
    type Error = cbor_event::Error;

    fn deserialize<R: std::io::BufRead>(reader: R) -> Result<Self, Self::Error> {
        Deserialize::deserialize(&mut Deserializer::from(reader))
    }
}

// **************************************************************************
// CBOR implementations
// **************************************************************************

impl cbor_event::se::Serialize for Block {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let serializer = serializer.write_array(cbor_event::Len::Len(2))?;
        match self {
            &Block::BoundaryBlock(ref gbh) => serializer.write_unsigned_integer(0)?.serialize(gbh),
            &Block::MainBlock(ref mbh) => serializer.write_unsigned_integer(1)?.serialize(mbh),
        }
    }
}
impl cbor_event::de::Deserialize for Block {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        match decode_sum_type(raw)? {
            0 => {
                let blk = cbor_event::de::Deserialize::deserialize(raw)?;
                Ok(Block::BoundaryBlock(blk))
            }
            1 => {
                let blk = cbor_event::de::Deserialize::deserialize(raw)?;
                Ok(Block::MainBlock(blk))
            }
            idx => Err(cbor_event::Error::CustomError(format!(
                "Unsupported Block: {}",
                idx
            ))),
        }
    }
}

impl<'a> cbor_event::se::Serialize for BlockHeaderView<'a> {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let serializer = serializer.write_array(cbor_event::Len::Len(2))?;
        match self {
            BlockHeaderView::Boundary(hdr) => serializer.write_unsigned_integer(0)?.serialize(hdr),
            BlockHeaderView::Normal(hdr) => serializer.write_unsigned_integer(1)?.serialize(hdr),
        }
    }
}

impl cbor_event::se::Serialize for BlockHeader {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let serializer = serializer.write_array(cbor_event::Len::Len(2))?;
        match self {
            &BlockHeader::BoundaryBlockHeader(ref gbh) => {
                serializer.write_unsigned_integer(0)?.serialize(gbh)
            }
            &BlockHeader::MainBlockHeader(ref mbh) => {
                serializer.write_unsigned_integer(1)?.serialize(mbh)
            }
        }
    }
}

impl cbor_event::de::Deserialize for BlockHeader {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        match decode_sum_type(raw)? {
            0 => {
                let blk = cbor_event::de::Deserialize::deserialize(raw)?;
                Ok(BlockHeader::BoundaryBlockHeader(blk))
            }
            1 => {
                let blk = cbor_event::de::Deserialize::deserialize(raw)?;
                Ok(BlockHeader::MainBlockHeader(blk))
            }
            idx => Err(cbor_event::Error::CustomError(format!(
                "Unsupported BlockHeader: {}",
                idx
            ))),
        }
    }
}

impl cbor_event::se::Serialize for BlockHeaders {
    fn serialize<'se, W: Write>(
        &self,
        serializer: &'se mut Serializer<W>,
    ) -> cbor_event::Result<&'se mut Serializer<W>> {
        let serializer = serializer
            .write_array(cbor_event::Len::Len(2))?
            .write_unsigned_integer(0)?;
        cbor_event::se::serialize_fixed_array(self.0.iter(), serializer)
    }
}
impl cbor_event::de::Deserialize for BlockHeaders {
    fn deserialize<R: BufRead>(raw: &mut Deserializer<R>) -> cbor_event::Result<Self> {
        match decode_sum_type(raw)? {
            0 => Ok(BlockHeaders(Vec::<BlockHeader>::deserialize(raw)?)),
            1 => Err(cbor_event::Error::CustomError(format!(
                "Server returned an error for Headers: {}",
                raw.text().unwrap()
            ))),
            idx => Err(cbor_event::Error::CustomError(format!(
                "Unsupported Headers: {}",
                idx
            ))),
        }
    }
}

#[cfg(test)]
mod test {
    use cbor_event::de::Deserializer;
    use std::io::Cursor;
    use util::hex;
    const MAINBLOCK_HEX: [u8; 408] = [
        0x82, 0x01, 0x85, 0x00, 0x58, 0x20, 0xc4, 0xe0, 0xfc, 0x3a, 0x4f, 0xfb, 0x31, 0x91, 0xf8,
        0x8b, 0x26, 0xa9, 0x83, 0x44, 0x53, 0xcb, 0xac, 0x0e, 0x6b, 0x9c, 0x8d, 0x8f, 0x7a, 0xe8,
        0x10, 0x69, 0x6b, 0xee, 0x57, 0x5d, 0x1d, 0x22, 0x84, 0x83, 0x01, 0x58, 0x20, 0x96, 0xd3,
        0x8c, 0x5a, 0xaf, 0xb8, 0x39, 0x45, 0x05, 0x11, 0xe1, 0xba, 0xe3, 0xb4, 0xec, 0xde, 0x21,
        0x58, 0x88, 0xde, 0xe3, 0x40, 0x35, 0x26, 0xe2, 0x37, 0x3d, 0x01, 0x6f, 0xdf, 0xdd, 0x1e,
        0x58, 0x20, 0x83, 0xac, 0x5d, 0x0d, 0x6a, 0xc0, 0xc0, 0x2a, 0xbf, 0x8c, 0x5a, 0xd7, 0x66,
        0xd0, 0x13, 0x58, 0x73, 0xca, 0x4a, 0xc5, 0x3d, 0xd5, 0x82, 0x18, 0x7c, 0x9a, 0xa1, 0x5a,
        0xa1, 0x49, 0xc0, 0xda, 0x82, 0x03, 0x58, 0x20, 0xc4, 0xe0, 0xfc, 0x3a, 0x4f, 0xfb, 0x31,
        0x91, 0xf8, 0x8b, 0x26, 0xa9, 0x83, 0x44, 0x53, 0xcb, 0xac, 0x0e, 0x6b, 0x9c, 0x8d, 0x8f,
        0x7a, 0xe8, 0x10, 0x69, 0x6b, 0xee, 0x57, 0x5d, 0x1d, 0x22, 0x58, 0x20, 0xc4, 0xe0, 0xfc,
        0x3a, 0x4f, 0xfb, 0x31, 0x91, 0xf8, 0x8b, 0x26, 0xa9, 0x83, 0x44, 0x53, 0xcb, 0xac, 0x0e,
        0x6b, 0x9c, 0x8d, 0x8f, 0x7a, 0xe8, 0x10, 0x69, 0x6b, 0xee, 0x57, 0x5d, 0x1d, 0x22, 0x58,
        0x20, 0xc4, 0xe0, 0xfc, 0x3a, 0x4f, 0xfb, 0x31, 0x91, 0xf8, 0x8b, 0x26, 0xa9, 0x83, 0x44,
        0x53, 0xcb, 0xac, 0x0e, 0x6b, 0x9c, 0x8d, 0x8f, 0x7a, 0xe8, 0x10, 0x69, 0x6b, 0xee, 0x57,
        0x5d, 0x1d, 0x22, 0x84, 0x82, 0x01, 0x18, 0x2a, 0x58, 0x40, 0x1c, 0x0c, 0x3a, 0xe1, 0x82,
        0x5e, 0x90, 0xb6, 0xdd, 0xda, 0x3f, 0x40, 0xa1, 0x22, 0xc0, 0x07, 0xe1, 0x00, 0x8e, 0x83,
        0xb2, 0xe1, 0x02, 0xc1, 0x42, 0xba, 0xef, 0xb7, 0x21, 0xd7, 0x2c, 0x1a, 0x5d, 0x36, 0x61,
        0xde, 0xb9, 0x06, 0x4f, 0x2d, 0x0e, 0x03, 0xfe, 0x85, 0xd6, 0x80, 0x70, 0xb2, 0xfe, 0x33,
        0xb4, 0x91, 0x60, 0x59, 0x65, 0x8e, 0x28, 0xac, 0x7f, 0x7f, 0x91, 0xca, 0x4b, 0x12, 0x81,
        0x18, 0x2a, 0x82, 0x00, 0x58, 0x40, 0xa9, 0x05, 0x22, 0x87, 0x4c, 0xcc, 0xf9, 0xa6, 0x7e,
        0x20, 0x90, 0x31, 0xfd, 0x9d, 0xfe, 0x37, 0xa8, 0x2f, 0xd9, 0x43, 0xde, 0xe6, 0x33, 0x00,
        0xaa, 0x82, 0x3c, 0xb9, 0x8e, 0x0f, 0x70, 0x4e, 0x91, 0x3f, 0x6e, 0x02, 0xb2, 0xaa, 0x0a,
        0x33, 0x69, 0x3e, 0x05, 0x2c, 0x15, 0xf4, 0x3a, 0xee, 0x24, 0x21, 0x64, 0xd2, 0x81, 0x2a,
        0x57, 0x2b, 0x27, 0x74, 0xc1, 0xb5, 0xad, 0xa8, 0x18, 0x01, 0x84, 0x83, 0x00, 0x01, 0x00,
        0x82, 0x6a, 0x63, 0x61, 0x72, 0x64, 0x61, 0x6e, 0x6f, 0x2d, 0x73, 0x6c, 0x00, 0xa0, 0x58,
        0x20, 0xc4, 0xe0, 0xfc, 0x3a, 0x4f, 0xfb, 0x31, 0x91, 0xf8, 0x8b, 0x26, 0xa9, 0x83, 0x44,
        0x53, 0xcb, 0xac, 0x0e, 0x6b, 0x9c, 0x8d, 0x8f, 0x7a, 0xe8, 0x10, 0x69, 0x6b, 0xee, 0x57,
        0x5d, 0x1d, 0x22,
    ];
    const GENESISBLOCK_HEX: [u8; 78] = [
        0x82, 0x00, 0x85, 0x00, 0x58, 0x20, 0xc4, 0xe0, 0xfc, 0x3a, 0x4f, 0xfb, 0x31, 0x91, 0xf8,
        0x8b, 0x26, 0xa9, 0x83, 0x44, 0x53, 0xcb, 0xac, 0x0e, 0x6b, 0x9c, 0x8d, 0x8f, 0x7a, 0xe8,
        0x10, 0x69, 0x6b, 0xee, 0x57, 0x5d, 0x1d, 0x22, 0x58, 0x20, 0xc4, 0xe0, 0xfc, 0x3a, 0x4f,
        0xfb, 0x31, 0x91, 0xf8, 0x8b, 0x26, 0xa9, 0x83, 0x44, 0x53, 0xcb, 0xac, 0x0e, 0x6b, 0x9c,
        0x8d, 0x8f, 0x7a, 0xe8, 0x10, 0x69, 0x6b, 0xee, 0x57, 0x5d, 0x1d, 0x22, 0x82, 0x01, 0x81,
        0x00, 0x81, 0xa0,
    ];

    const MAINBLOCK_HASH: &str = "12d339c93f216d1b775297dcf465428aa43f73518466bf72fc6413448ec27069";
    const GENESIS_HASH: &str = "0027f90a735237e2555b418ac4e02d35daf75945aad6253c7ac0bc7b121f974b";

    fn check_blockheader_serialization(header_raw: &[u8], hash: &str) {
        let mut de = Deserializer::from(Cursor::new(header_raw));
        let header: super::BlockHeader = de.deserialize().unwrap();
        let got_raw = cbor!(&header).unwrap();
        assert_eq!(hex::encode(header_raw), hex::encode(&got_raw[..]));
        let got_hash = header.compute_hash();
        let got_hex = hex::encode(got_hash.as_ref());
        assert_eq!(hash, got_hex)
    }

    #[test]
    fn check_boundary_block() {
        check_blockheader_serialization(&GENESISBLOCK_HEX[..], GENESIS_HASH);
    }

    #[test]
    fn check_main_block() {
        check_blockheader_serialization(&MAINBLOCK_HEX[..], MAINBLOCK_HASH);
    }
}

#[cfg(test)]
#[cfg(feature = "with-bench")]
mod bench {
    use super::{Block, HeaderHash};
    use cbor_event::{self, de::RawCbor};
    use test;

    const BLOCK: &'static [u8] = &[
        130, 1, 131, 133, 26, 45, 150, 74, 9, 88, 32, 62, 112, 94, 154, 162, 127, 229, 78, 44, 102,
        42, 10, 90, 168, 12, 54, 11, 212, 124, 226, 75, 185, 66, 157, 250, 79, 223, 23, 12, 45,
        237, 129, 132, 131, 3, 88, 32, 10, 86, 22, 140, 149, 198, 120, 31, 227, 126, 104, 83, 155,
        108, 239, 136, 206, 225, 180, 114, 225, 210, 154, 123, 227, 237, 73, 121, 41, 194, 156, 61,
        88, 32, 79, 163, 255, 228, 159, 194, 53, 158, 174, 181, 226, 78, 112, 192, 122, 233, 82, 0,
        12, 57, 201, 15, 166, 113, 149, 40, 182, 171, 39, 208, 57, 63, 130, 3, 88, 32, 211, 106,
        38, 25, 166, 114, 73, 70, 4, 225, 27, 180, 71, 203, 207, 82, 49, 233, 242, 186, 37, 194,
        22, 145, 119, 237, 201, 65, 189, 80, 173, 108, 88, 32, 175, 192, 218, 100, 24, 59, 242,
        102, 79, 61, 78, 236, 114, 56, 213, 36, 186, 96, 127, 174, 234, 178, 79, 193, 0, 235, 134,
        29, 186, 105, 151, 27, 88, 32, 78, 102, 40, 12, 217, 77, 89, 16, 114, 52, 155, 236, 10, 48,
        144, 165, 58, 169, 69, 86, 46, 251, 109, 8, 213, 110, 83, 101, 75, 14, 64, 152, 132, 130,
        1, 25, 55, 178, 88, 64, 27, 201, 122, 47, 224, 44, 41, 120, 128, 206, 142, 207, 217, 151,
        254, 76, 30, 192, 158, 225, 15, 238, 238, 159, 104, 103, 96, 22, 107, 5, 40, 29, 98, 131,
        70, 143, 253, 147, 190, 203, 12, 149, 108, 205, 221, 100, 45, 249, 177, 36, 76, 145, 89,
        17, 24, 95, 164, 147, 85, 246, 242, 43, 250, 185, 129, 25, 139, 254, 130, 2, 130, 132, 0,
        88, 64, 27, 201, 122, 47, 224, 44, 41, 120, 128, 206, 142, 207, 217, 151, 254, 76, 30, 192,
        158, 225, 15, 238, 238, 159, 104, 103, 96, 22, 107, 5, 40, 29, 98, 131, 70, 143, 253, 147,
        190, 203, 12, 149, 108, 205, 221, 100, 45, 249, 177, 36, 76, 145, 89, 17, 24, 95, 164, 147,
        85, 246, 242, 43, 250, 185, 88, 64, 97, 38, 26, 149, 183, 97, 62, 230, 191, 32, 103, 218,
        215, 123, 112, 52, 151, 41, 176, 197, 13, 87, 188, 28, 243, 13, 224, 219, 74, 30, 115, 168,
        133, 208, 5, 74, 247, 194, 63, 198, 195, 121, 25, 219, 164, 28, 96, 42, 87, 226, 208, 249,
        50, 154, 121, 84, 184, 103, 51, 141, 111, 178, 201, 69, 88, 64, 224, 62, 98, 240, 131, 223,
        85, 118, 54, 14, 96, 163, 46, 34, 187, 176, 123, 60, 141, 244, 252, 171, 128, 121, 241,
        214, 246, 26, 243, 149, 77, 36, 43, 168, 160, 101, 22, 195, 149, 147, 159, 36, 9, 111, 61,
        241, 78, 16, 58, 125, 156, 43, 128, 166, 138, 147, 99, 207, 31, 39, 199, 164, 227, 7, 88,
        64, 42, 100, 242, 153, 199, 254, 84, 67, 51, 137, 202, 116, 199, 207, 142, 44, 53, 255, 70,
        58, 54, 18, 240, 140, 181, 106, 206, 181, 158, 252, 117, 219, 71, 72, 173, 124, 18, 247,
        65, 137, 253, 229, 115, 105, 145, 72, 224, 252, 249, 120, 242, 145, 208, 193, 222, 166,
        247, 245, 217, 138, 12, 177, 27, 5, 132, 131, 0, 0, 0, 130, 106, 99, 97, 114, 100, 97, 110,
        111, 45, 115, 108, 1, 160, 88, 32, 75, 169, 42, 163, 32, 198, 10, 204, 154, 215, 185, 166,
        79, 46, 218, 85, 196, 210, 236, 40, 230, 4, 250, 241, 134, 112, 139, 79, 12, 78, 142, 223,
        132, 159, 130, 131, 159, 130, 0, 216, 24, 88, 36, 130, 88, 32, 196, 201, 143, 96, 200, 75,
        77, 220, 200, 197, 238, 183, 77, 246, 208, 230, 58, 170, 131, 97, 127, 141, 150, 72, 27,
        66, 38, 76, 115, 159, 62, 152, 1, 255, 159, 130, 130, 216, 24, 88, 66, 131, 88, 28, 109,
        41, 37, 255, 14, 12, 164, 98, 33, 206, 227, 159, 180, 245, 102, 218, 174, 143, 145, 218,
        231, 243, 166, 197, 27, 62, 176, 105, 161, 1, 88, 30, 88, 28, 156, 233, 149, 81, 19, 219,
        223, 184, 26, 202, 202, 89, 7, 131, 173, 125, 28, 221, 19, 150, 254, 144, 90, 50, 43, 6,
        46, 42, 0, 26, 132, 103, 17, 249, 27, 0, 0, 0, 2, 115, 156, 125, 31, 130, 130, 216, 24, 88,
        66, 131, 88, 28, 198, 65, 169, 229, 147, 191, 175, 29, 108, 155, 53, 49, 126, 7, 55, 98,
        103, 184, 234, 16, 227, 110, 150, 26, 14, 82, 238, 70, 161, 1, 88, 30, 88, 28, 202, 62, 85,
        60, 156, 99, 197, 85, 63, 78, 15, 67, 192, 251, 32, 17, 249, 167, 65, 253, 190, 50, 79,
        220, 219, 107, 108, 118, 0, 26, 85, 144, 176, 113, 27, 0, 0, 0, 7, 115, 89, 64, 0, 255,
        160, 129, 130, 0, 216, 24, 88, 133, 130, 88, 64, 52, 33, 34, 217, 196, 36, 81, 143, 53, 26,
        6, 104, 73, 172, 143, 127, 82, 47, 14, 92, 238, 235, 183, 157, 91, 219, 210, 229, 195, 239,
        106, 129, 194, 10, 146, 48, 16, 248, 89, 121, 19, 60, 81, 167, 56, 39, 239, 167, 204, 54,
        186, 230, 48, 7, 199, 49, 166, 61, 229, 28, 205, 153, 88, 151, 88, 64, 185, 100, 27, 141,
        91, 107, 2, 249, 90, 103, 122, 45, 68, 15, 249, 66, 194, 175, 190, 156, 30, 207, 74, 146,
        17, 80, 210, 145, 249, 144, 1, 199, 112, 93, 142, 235, 71, 241, 179, 88, 21, 156, 169, 97,
        55, 68, 226, 174, 162, 166, 164, 195, 143, 123, 193, 189, 172, 32, 135, 145, 102, 251, 150,
        13, 130, 131, 159, 130, 0, 216, 24, 88, 36, 130, 88, 32, 254, 249, 136, 177, 233, 204, 49,
        255, 41, 187, 1, 103, 73, 165, 67, 240, 118, 89, 173, 97, 230, 119, 102, 61, 159, 29, 117,
        241, 94, 249, 108, 155, 0, 255, 159, 130, 130, 216, 24, 88, 66, 131, 88, 28, 253, 232, 220,
        241, 35, 230, 18, 203, 65, 245, 5, 98, 140, 94, 242, 66, 119, 141, 108, 102, 86, 53, 183,
        246, 7, 162, 109, 54, 161, 1, 88, 30, 88, 28, 202, 62, 85, 60, 156, 99, 197, 54, 189, 86,
        50, 67, 221, 70, 75, 55, 45, 223, 197, 30, 135, 48, 245, 33, 52, 83, 215, 212, 0, 26, 24,
        39, 231, 206, 27, 0, 0, 68, 42, 61, 49, 72, 182, 130, 130, 216, 24, 88, 66, 131, 88, 28,
        164, 97, 148, 87, 168, 130, 95, 44, 96, 48, 61, 203, 225, 14, 55, 237, 114, 162, 20, 215,
        22, 208, 80, 228, 196, 56, 148, 92, 161, 1, 88, 30, 88, 28, 202, 62, 85, 60, 156, 99, 197,
        127, 196, 34, 34, 67, 116, 107, 58, 95, 49, 200, 247, 77, 85, 7, 56, 21, 66, 246, 127, 127,
        0, 26, 196, 69, 157, 80, 26, 0, 149, 137, 64, 255, 160, 129, 130, 0, 216, 24, 88, 133, 130,
        88, 64, 155, 184, 74, 86, 173, 97, 208, 223, 214, 4, 126, 202, 70, 59, 110, 105, 26, 139,
        232, 220, 6, 77, 0, 78, 92, 155, 121, 117, 33, 85, 182, 121, 10, 167, 156, 202, 239, 176,
        76, 171, 95, 99, 108, 212, 143, 127, 147, 149, 146, 109, 86, 95, 231, 127, 215, 36, 197,
        237, 231, 220, 62, 35, 150, 220, 88, 64, 35, 117, 37, 48, 190, 106, 102, 239, 185, 196,
        100, 118, 185, 43, 127, 201, 118, 155, 180, 45, 51, 210, 22, 138, 191, 235, 42, 194, 88,
        249, 50, 63, 179, 81, 60, 152, 42, 13, 78, 131, 156, 226, 150, 18, 165, 110, 168, 172, 166,
        55, 169, 13, 135, 99, 93, 217, 37, 254, 29, 110, 149, 228, 107, 2, 130, 131, 159, 130, 0,
        216, 24, 88, 36, 130, 88, 32, 199, 231, 1, 92, 250, 75, 68, 18, 224, 185, 52, 234, 204,
        157, 167, 1, 160, 181, 154, 237, 242, 130, 41, 43, 77, 47, 164, 45, 158, 112, 122, 97, 0,
        255, 159, 130, 130, 216, 24, 88, 66, 131, 88, 28, 163, 218, 5, 111, 245, 194, 8, 14, 101,
        50, 34, 31, 29, 115, 41, 218, 45, 53, 104, 161, 65, 111, 93, 157, 220, 88, 50, 119, 161, 1,
        88, 30, 88, 28, 212, 214, 100, 87, 247, 230, 137, 18, 233, 14, 67, 83, 249, 72, 243, 110,
        203, 204, 34, 103, 73, 150, 185, 178, 143, 128, 107, 78, 0, 26, 181, 179, 13, 99, 27, 0, 0,
        0, 75, 49, 142, 246, 128, 130, 130, 216, 24, 88, 66, 131, 88, 28, 6, 251, 79, 181, 192,
        149, 80, 229, 54, 76, 214, 94, 36, 111, 110, 21, 71, 201, 75, 12, 182, 244, 84, 255, 253,
        170, 124, 24, 161, 1, 88, 30, 88, 28, 202, 62, 85, 60, 156, 99, 197, 120, 165, 214, 82, 67,
        73, 247, 123, 106, 164, 183, 94, 5, 188, 198, 45, 79, 156, 4, 67, 62, 0, 26, 125, 51, 214,
        184, 27, 0, 0, 1, 27, 15, 159, 21, 146, 255, 160, 129, 130, 0, 216, 24, 88, 133, 130, 88,
        64, 38, 89, 182, 201, 162, 103, 59, 81, 234, 18, 97, 102, 246, 232, 45, 127, 221, 63, 182,
        36, 193, 177, 115, 84, 201, 172, 245, 43, 114, 161, 80, 197, 102, 139, 116, 190, 240, 163,
        235, 16, 61, 190, 118, 12, 43, 129, 109, 238, 119, 3, 78, 105, 197, 20, 30, 186, 112, 158,
        24, 1, 27, 208, 240, 201, 88, 64, 50, 40, 38, 231, 87, 89, 38, 206, 149, 84, 138, 12, 206,
        233, 146, 156, 60, 39, 6, 111, 20, 177, 185, 25, 145, 135, 65, 46, 153, 206, 183, 141, 72,
        223, 211, 154, 88, 187, 246, 84, 170, 54, 124, 84, 116, 144, 130, 40, 237, 254, 121, 108,
        212, 242, 177, 213, 162, 150, 34, 1, 145, 220, 229, 1, 255, 130, 3, 217, 1, 2, 128, 159,
        255, 130, 128, 159, 255, 129, 160,
    ];

    #[bench]
    fn decode_block_cbor_raw(b: &mut test::Bencher) {
        b.iter(|| {
            let mut raw = RawCbor::from(BLOCK);
            let _: Block = cbor_event::Deserialize::deserialize(&mut raw).unwrap();
        })
    }

    /*
    #[bench]
    #[ignore]
    fn encode_block_cbor(b: &mut test::Bencher) {
        let blk : Block = cbor::decode_from_cbor(BLOCK).unwrap();
        b.iter(|| {
            let _ : Vec<u8> = cbor::encode_to_cbor(&blk).unwrap();
        })
    }

    #[bench]
    fn encode_blockheader_cbor(b: &mut test::Bencher) {
        let blk : Block = cbor::decode_from_cbor(BLOCK).unwrap();
        let hdr = blk.get_header();
        b.iter(|| { let _ : Vec<u8> = cbor::encode_to_cbor(&hdr).unwrap(); } )
    }

    #[bench]
    fn compute_header_hash(b: &mut test::Bencher) {
        let blk : Block = cbor::decode_from_cbor(BLOCK).unwrap();
        let hdr = blk.get_header();
        b.iter(|| { hdr.compute_hash() } )
    }

    #[bench]
    fn get_block_hash(b: &mut test::Bencher) {
        b.iter(|| {
            let blk : Block = cbor::decode_from_cbor(BLOCK).unwrap();
            let hdr = blk.get_header();
            hdr.compute_hash();
        })
    }
    */
}
