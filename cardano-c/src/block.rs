use cardano::block;
use std::ptr;
use types::{BlockPtr, CardanoResult, SignedTransactionPtr};

#[no_mangle]
pub extern "C" fn cardano_raw_block_decode(
    bytes: *const u8,
    size: usize,
    out: *mut BlockPtr,
) -> CardanoResult {
    let slice = unsafe { std::slice::from_raw_parts(bytes, size) };
    let raw_block = block::block::RawBlock::from_dat(slice.to_vec());
    let block = match raw_block.decode() {
        Ok(b) => b,
        Err(_) => return CardanoResult::failure(),
    };
    let pointer = Box::into_raw(Box::new(block));
    unsafe { ptr::write(out, pointer) };
    CardanoResult::success()
}

#[no_mangle]
pub extern "C" fn cardano_block_delete(block: BlockPtr) {
    unsafe { Box::from_raw(block) };
}

#[no_mangle]
pub extern "C" fn cardano_block_get_transactions(
    block: BlockPtr,
    out_pointer: *mut *mut *const cardano::tx::TxAux,
    size: *mut usize,
) -> CardanoResult {
    let block = unsafe { block.as_mut() }.expect("Not a NULL PTR");

    use cardano::block::block::Block::BoundaryBlock;
    use cardano::block::block::Block::MainBlock;

    let payload = match block {
        BoundaryBlock(_) => return CardanoResult::failure(),
        MainBlock(ref blk) => &blk.body.tx,
    };

    let mut txs = payload
        .iter()
        .map(|tx| tx as *const cardano::tx::TxAux)
        .collect::<Vec<*const cardano::tx::TxAux>>()
        .into_boxed_slice();

    let pointer = txs.as_mut_ptr();
    let length = txs.len();

    std::mem::forget(txs);

    unsafe { ptr::write(out_pointer, pointer) };
    unsafe { ptr::write(size, length) };

    CardanoResult::success()
}

#[no_mangle]
pub extern "C" fn cardano_block_delete_transactions(
    pointer: *mut SignedTransactionPtr,
    size: usize,
) {
    unsafe {
        let slice = std::slice::from_raw_parts_mut(pointer, size);
        Box::from_raw(slice);
    };
}
