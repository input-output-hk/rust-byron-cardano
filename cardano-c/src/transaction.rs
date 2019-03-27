use cardano::coin::Coin;
use cardano::config::ProtocolMagic;
use cardano::fee::{self, LinearFee};
use cardano::tx::{self, TxId, TxInWitness};
use cardano::txbuild::{Error, TxBuilder, TxFinalized};
use cardano::txutils::OutputPolicy;
use cardano::util::try_from_slice::TryFromSlice;
use std::{ptr, slice};
use types::*;

#[no_mangle]
pub extern "C" fn cardano_transaction_output_ptr_new(
    c_txid: *mut u8,
    index: u32,
) -> TransactionOutputPointerPtr {
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
pub extern "C" fn cardano_transaction_output_new(
    c_addr: AddressPtr,
    value: u64,
) -> TransactionOutputPtr {
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
pub extern "C" fn cardano_transaction_builder_add_output(
    tb: TransactionBuilderPtr,
    c_out: TransactionOutputPtr,
) {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let out = unsafe { c_out.as_ref() }.expect("Not a NULL PTR");
    builder.add_output_value(out)
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_add_input(
    tb: TransactionBuilderPtr,
    c_txo: TransactionOutputPointerPtr,
    value: u64,
) -> CardanoTransactionErrorCode {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let txo = unsafe { c_txo.as_ref() }.expect("Not a NULL PTR");
    if let Ok(coin) = Coin::new(value) {
        builder.add_input(txo, coin);
        CardanoTransactionErrorCode::success()
    } else {
        CardanoTransactionErrorCode::coin_out_of_bounds()
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_add_change_addr(
    tb: TransactionBuilderPtr,
    change_addr: AddressPtr,
) -> CardanoResult {
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
pub extern "C" fn cardano_transaction_builder_fee(tb: TransactionBuilderPtr) -> u64 {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let fee_algo = LinearFee::default();

    if let Ok(fee) = builder.calculate_fee(&fee_algo) {
        u64::from(fee.to_coin())
    } else {
        // failed to calculate transaction fee, return zero
        u64::from(fee::Fee::new(Coin::zero()).to_coin())
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_balance(
    tb: TransactionBuilderPtr,
    out: *mut *mut Balance,
) -> CardanoTransactionErrorCode {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let balance: Box<Balance> = match builder.balance(&LinearFee::default()) {
        Ok(v) => Box::new(v.into()),
        Err(e) => return e.into(),
    };

    unsafe { ptr::write(out, Box::into_raw(balance)) };

    CardanoTransactionErrorCode::success()
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_balance_without_fees(
    tb: TransactionBuilderPtr,
    out: *mut *mut Balance,
) -> CardanoTransactionErrorCode {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let balance: Box<Balance> = match builder.balance_without_fees() {
        Ok(v) => Box::new(v.into()),
        Err(e) => return e.into(),
    };

    unsafe { ptr::write(out, Box::into_raw(balance)) };

    CardanoTransactionErrorCode::success()
}

#[no_mangle]
pub extern "C" fn cardano_transaction_balance_delete(balance: *mut Balance) {
    let _ = unsafe { Box::from_raw(balance) };
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_get_input_total(
    tb: TransactionBuilderPtr,
    out: *mut u64,
) -> CardanoTransactionErrorCode {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let result: u64 = match builder.get_input_total() {
        Ok(number) => number.into(),
        Err(e) => return e.into(),
    };
    unsafe { ptr::write(out, result) };
    CardanoTransactionErrorCode::success()
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_get_output_total(
    tb: TransactionBuilderPtr,
    out: *mut u64,
) -> CardanoTransactionErrorCode {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let result: u64 = match builder.get_output_total() {
        Ok(number) => number.into(),
        Err(e) => return e.into(),
    };
    unsafe { ptr::write(out, result) };
    CardanoTransactionErrorCode::success()
}

#[no_mangle]
pub extern "C" fn cardano_transaction_builder_finalize(
    tb: TransactionBuilderPtr,
    tx_out: *mut TransactionPtr,
) -> CardanoTransactionErrorCode {
    let builder = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    match builder.clone().make_tx() {
        Ok(tx) => {
            let boxed = Box::new(tx);
            unsafe { ptr::write(tx_out, Box::into_raw(boxed)) };
            CardanoTransactionErrorCode::success()
        }
        Err(Error::TxInvalidNoInput) => CardanoTransactionErrorCode::no_inputs(),
        Err(Error::TxInvalidNoOutput) => CardanoTransactionErrorCode::no_outputs(),
        _ => panic!("Shouldn't happen"),
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_delete(tx: TransactionPtr) {
    unsafe { Box::from_raw(tx) };
}

#[no_mangle]
pub extern "C" fn cardano_transaction_finalized_new(
    c_tx: TransactionPtr,
) -> TransactionFinalizedPtr {
    let tx = unsafe { c_tx.as_ref() }.expect("Not a NULL PTR");
    let finalized = TxFinalized::new(tx.clone());
    let b = Box::new(finalized);
    Box::into_raw(b)
}

#[no_mangle]
pub extern "C" fn cardano_transaction_finalized_delete(c_txf: TransactionFinalizedPtr) {
    unsafe { Box::from_raw(c_txf) };
}

#[no_mangle]
pub extern "C" fn cardano_transaction_finalized_add_witness(
    tb: TransactionFinalizedPtr,
    c_xprv: XPrvPtr,
    protocol_magic: ProtocolMagic,
    c_txid: *mut u8,
) -> CardanoTransactionErrorCode {
    let tf = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    let xprv = unsafe { c_xprv.as_ref() }.expect("Not a NULL PTR");
    let txid_slice = unsafe { slice::from_raw_parts(c_txid, TxId::HASH_SIZE) };
    let txid = TxId::try_from_slice(txid_slice).unwrap();

    let witness = TxInWitness::new(protocol_magic, xprv, &txid);
    if let Ok(()) = tf.add_witness(witness) {
        CardanoTransactionErrorCode::success()
    } else {
        CardanoTransactionErrorCode::signatures_exceeded()
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_finalized_output(
    tb: TransactionFinalizedPtr,
    txaux_out: *mut SignedTransactionPtr,
) -> CardanoTransactionErrorCode {
    let tf = unsafe { tb.as_mut() }.expect("Not a NULL PTR");
    match tf.clone().make_txaux() {
        Ok(txaux) => {
            let boxed = Box::new(txaux);
            unsafe { ptr::write(txaux_out, Box::into_raw(boxed)) };
            CardanoTransactionErrorCode::success()
        }
        Err(Error::TxSignaturesMismatch) => CardanoTransactionErrorCode::signature_mismatch(),
        Err(Error::TxOverLimit(_)) => CardanoTransactionErrorCode::over_limit(),
        _ => panic!("Shouldn't happen"),
    }
}

#[no_mangle]
pub extern "C" fn cardano_transaction_signed_delete(txaux: SignedTransactionPtr) {
    unsafe { Box::from_raw(txaux) };
}
