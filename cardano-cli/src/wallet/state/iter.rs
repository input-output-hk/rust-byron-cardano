use super::super::super::blockchain;
use cardano::{block::Block, tx::TxAux};

use indicatif::ProgressBar;
use super::ptr::StatePtr;

pub struct TransactionIterator<'a>
{
    block_iterator: blockchain::iter::Iter<'a>,
    progress: ProgressBar,

    current_tx: Option<(Block, usize)>,
}
impl<'a> TransactionIterator<'a> {
    fn mk_tx(&self) -> Option<(StatePtr, TxAux)> {
        if let Some((block, idx)) = &self.current_tx {
            let hdr = block.get_header();
            let date = hdr.get_blockdate();
            let hh = hdr.compute_hash();
            let ptr = StatePtr::new(date, hh);

            block.get_transactions().and_then(|txpayload| {
                if let Some(ref txaux) = txpayload.get(*idx) {
                    Some((ptr, (*txaux).clone()))
                } else {
                    None
                }
            })
        } else {
            None
        }
    }

    fn skip_no_transactions(&mut self) -> blockchain::iter::Result<()> {
        self.current_tx = None;
        loop {
            if let Some(raw_block) = self.block_iterator.next() {
                self.progress.inc(1);
                let block = raw_block?.decode()?;
                if block.has_transactions() {
                    self.current_tx = Some((block, 0));
                    break;
                }
            } else {
                break;
            }
        }
        Ok(())
    }
    pub fn new(progress: ProgressBar, block_iterator: blockchain::iter::Iter<'a>) -> Self {
        TransactionIterator {
            block_iterator: block_iterator,
            progress: progress,
            current_tx: None
        }
    }
}
impl<'a> Iterator for TransactionIterator<'a> {
    type Item = blockchain::iter::Result<(StatePtr, TxAux)>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.mk_tx() {
            None => {
                if let Err(err) = self.skip_no_transactions() {
                    return Some(Err(err));
                }

                match self.mk_tx() {
                    None => {
                        self.progress.finish();
                        None
                    },
                    Some(r) => {
                        if let Some(( _, ref mut idx)) = &mut self.current_tx {
                            *idx += 1;
                        }
                        Some(Ok(r))
                    }
                }
            },
            Some(r) => {
                if let Some((_, ref mut idx)) = &mut self.current_tx {
                    *idx += 1;
                }
                Some(Ok(r))
            }
        }
    }
}
