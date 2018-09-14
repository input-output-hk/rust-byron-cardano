use cardano::hdwallet;
use cardano::fee::{self, LinearFee};
use cardano::coin::Coin;
use cardano::config::ProtocolMagic;
use cardano::util::try_from_slice::TryFromSlice;
use cardano::txbuild::{TxBuilder, TxFinalized};
use cardano::tx::{self, TxInWitness, TxId};
use cardano::txutils::OutputPolicy;
use std::{
    ffi,
    ptr,
    slice,
    os::raw::{c_char, c_int},
};
use types::*;

#[no_mangle]
pub extern "C" fn cardano_transaction_output_ptr_new(c_txid: *mut u8, index: u32) -> TransactionOutputPointerPtr {
    let txid_slice = unsafe { slice::from_raw_parts(c_txid, TxId::HASH_SIZE) };
    let txid = TxId::try_from_slice(txid_slice).unwrap();
    let txo = tx::TxoPointer::new(txid, index);
    let b = Box::new(txo);
    Box::into_raw(b)
}

#[no_mangle]
pub extern "C" fn cardano_transaction_output_ptr_delete(txo: TransactionOutputPointerPtr) {
    unsafe { Box::from_raw(txo) };
}

#[no_mangle]
pub extern "C" fn cardano_transaction_output_new(c_addr: AddressPtr, value: u64) -> TransactionOutputPtr {
    let address = unsafe { c_addr.as_ref() }.expect("Not a NULL PTR");
    if let Ok(coin) = Coin::new(value) {
        let txout = tx::TxOut::new(address.clone(), coin);
        let b = Box::new(txout);
        Box::into_raw(b)
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_output_delete(output: TransactionOutputPtr) {
    unsafe { Box::from_raw(output) };
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_new() -> TransactionBuilderPtr {
    let builder = TxBuilder::new();
    let b = Box::new(builder);
    Box::into_raw(b)
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_delete(tb: TransactionBuilderPtr) {
    unsafe { Box::from_raw(tb) };
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_add_output(tb: TransactionBuilderPtr, c_out: TransactionOutputPtr) {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let out = unsafe { c_out.as_ref() }.expect("Not a NULL PTR");
    builder.add_output_value(out)
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_add_input(tb: TransactionBuilderPtr, c_txo: TransactionOutputPointerPtr, value: u64) -> CardanoResult {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let txo = unsafe { c_txo.as_ref() }.expect("Not a NULL PTR");
    if let Ok(coin) = Coin::new(value) {
        builder.add_input(txo, coin);
        CardanoResult::success()
    } else {
        CardanoResult::failure()
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_add_change_addr(tb: TransactionBuilderPtr, change_addr: AddressPtr) -> CardanoResult {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let addr = unsafe { change_addr.as_ref() }.expect("Not a NULL PTR");
    let fee = LinearFee::default();

    let output_policy = OutputPolicy::One(addr.clone());
    if let Ok(_) = builder.add_output_policy(&fee, &output_policy) {
        CardanoResult::success()
    } else {
        CardanoResult::failure()
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_finalize(tb: TransactionBuilderPtr) -> TransactionPtr {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    if let Ok(tx) = builder.clone().make_tx() {
        let b = Box::new(tx);
        Box::into_raw(b)
    } else {
        ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_finalized_new(c_tx: TransactionPtr) -> TransactionFinalizedPtr {
    let tx = unsafe { c_tx.as_ref() }.expect("Not a NULL PTR");
    let finalized = TxFinalized::new(tx.clone());
    let b = Box::new(finalized);
    Box::into_raw(b)
}

#[no_mangle]
pub extern "C" fn cardano_transaction_finalized_add_witness(tb: TransactionFinalizedPtr, c_xprv: XPrvPtr, protocol_magic: ProtocolMagic, c_txid: *mut u8) -> CardanoResult {
    let tf = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let xprv = unsafe { c_xprv.as_ref() }.expect("Not a NULL PTR");
    let txid_slice = unsafe { slice::from_raw_parts(c_txid, TxId::HASH_SIZE) };
    let txid = TxId::try_from_slice(txid_slice).unwrap();

    let witness = TxInWitness::new(protocol_magic, xprv, &txid);
    if let Ok(()) = tf.add_witness(witness) {
        CardanoResult::success()
    } else {
        CardanoResult::failure()
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_finalized_output(tb: TransactionFinalizedPtr) -> SignedTransactionPtr {
    let tf = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    if let Ok(txaux) = tf.clone().make_txaux() {
        let b = Box::new(txaux);
        Box::into_raw(b)
    } else {
        ptr::null_mut()
    }
}
