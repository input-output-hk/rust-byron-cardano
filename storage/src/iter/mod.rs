pub mod epoch;
mod range;
mod reverse;

pub use self::range::Range;

/// Constructs a `Range` iterator.
///
/// This function is meant for internal use.
/// It's more convenient to use the `range` method of `Storage`.
pub use self::range::iter as range_iter;

pub use self::reverse::ReverseIter;

/// Constructs a `ReverseIter` iterator.
///
/// This function is meant for internal use.
/// It's more convenient to use the `reverse_from` method of `Storage`.
pub use self::reverse::iter as reverse_iter;

use super::Result;

use super::{block_location, block_read_location, BlockLocation, Storage};

use cardano::block::{Block, RawBlock};
use storage_units::hash::BlockHash;

enum IteratorType<'a> {
    Epoch(epoch::Epochs<'a>, Option<epoch::Iter>),
    Loose(&'a Storage, Range),
}
impl<'a> IteratorType<'a> {
    fn is_loose(&self) -> bool {
        match self {
            IteratorType::Loose(_, _) => true,
            _ => false,
        }
    }
}
impl<'a> Iterator for IteratorType<'a> {
    type Item = Result<RawBlock>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IteratorType::Epoch(ref mut epochs, ref mut opt_iter) => {
                if opt_iter.is_none() {
                    *opt_iter = match epochs.next() {
                        None => None,
                        Some(Ok(v)) => Some(v),
                        Some(Err(err)) => { return Some(Err(err)); }
                    };
                }

                let res = if let Some(ref mut iter) = opt_iter {
                    iter.next()
                } else {
                    None
                };

                match res {
                    None => {
                        *opt_iter = match epochs.next() {
                            None => None,
                            Some(Ok(v)) => Some(v),
                            Some(Err(err)) => {
                                return Some(Err(err));
                            }
                        };
                        if let Some(ref mut iter) = opt_iter {
                            iter.next()
                        } else {
                            None
                        }
                    }
                    Some(res) => { Some(res) }
                }
            }
            IteratorType::Loose(ref storage, ref mut range) => {
                if let Some(bh) = range.next() {
                    let location = BlockLocation::Loose;
                    Some(Ok(block_read_location(&storage, &location, &bh.into()).unwrap()))
                } else {
                    None
                }
            }
        }
    }
}

#[deprecated]
pub struct Iter<'a> {
    storage: &'a Storage,

    initialised: bool,

    starting_from: BlockHash,
    ending_at: BlockHash,

    last_known_block_hash: Option<BlockHash>,

    iterator: IteratorType<'a>
}

#[allow(deprecated)]
impl<'a> Iter<'a> {
    #[deprecated]
    pub fn new<H>(
        storage: &'a Storage,
        from: H,
        to: H,
    ) -> Result<Self>
    where
        H: Into<BlockHash>
    {
        Self::new_internal(storage, from.into(), to.into())
    }

    fn new_internal(
        storage: &'a Storage,
        from: BlockHash,
        to: BlockHash,
    ) -> Result<Self> {
        let iterator = match block_location(&storage, &from)? {
            BlockLocation::Loose => {
                let mut range = range_iter(storage, from, to)?;
                IteratorType::Loose(storage, range)
            }
            location => {
                let block_header = block_read_location(&storage, &location, &from)
                    .unwrap()
                    .decode()?
                    .get_header();
                let block_date = block_header.get_blockdate();

                let epochs = epoch::Epochs::new(&storage.config).from_epoch(block_date.get_epochid());
                let mut iterator = IteratorType::Epoch(epochs, None);

                iterator
            }
        };

        let iter = Iter {
            storage: storage,
            initialised: false,
            starting_from: from,
            ending_at: to,
            last_known_block_hash: None,
            iterator: iterator
        };

        Ok(iter)
    }
}

#[allow(deprecated)]
impl<'a> Iterator for Iter<'a> {
    type Item = Result<(RawBlock, Block)>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref hh) = self.last_known_block_hash {
            if hh == &self.ending_at { return None; }
        }

        if ! self.initialised {
            self.initialised = true;

            let mut next = None;
            for block in self.iterator.next() {
                match block {
                    Err(err) => return Some(Err(err)),
                    Ok(raw_block) => {
                        let block = raw_block.decode().unwrap();
                        let hh = block.get_header().compute_hash();
                        let end = *hh == self.starting_from;
                        next = Some(Ok((raw_block, block)));
                        self.last_known_block_hash = Some(*hh);
                        if end { break; }
                    }
                }
            }

            next
        } else {
            match self.iterator.next() {
                None => {
                    if !self.iterator.is_loose() {
                        let mut range = range_iter(
                            &self.storage,
                            self.last_known_block_hash.clone().unwrap(),
                            self.ending_at.clone()
                        ).unwrap(); // TODO
                        range.next(); // remove the last known block hash (it was the one in the last epoch)
                        self.iterator = IteratorType::Loose(&self.storage, range);
                        self.next()
                    } else {
                        None
                    }
                },
                Some(Err(err)) => Some(Err(err)),
                Some(Ok(raw_block)) => {
                    let block = raw_block.decode().unwrap();
                    let hh = block.get_header().compute_hash();
                    self.last_known_block_hash = Some(*hh);
                    Some(Ok((raw_block, block)))
                }
            }
        }
    }
}
