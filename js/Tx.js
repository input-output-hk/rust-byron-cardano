import RustModule from './RustModule';
import { newArray, newArray0, copyArray } from './utils/arrays';
import { apply } from './utils/functions';
import { base16 } from './utils/strings';

/**
 * Create a TxIn from the given TxId and the Index
 *
 * @param module - the WASM module that is used for crypto operations
 * @param txid - the txid to get the amount from (array of bytes, not hexadecimal version)
 * @param index - the index in the tx pointed by the txid to get the address and money from
 * @returns {*} - a txin (encoded in cbor)
 */
export const newTxIn = (module, txid, index) => {
        const buftxid = newArray(module, txid);
        const buftxin = newArray0(module, 1024);

        let rsz = module.wallet_txin_create(buftxid, index, buftxin);
        let txin = copyArray(module, buftxin, rsz);

        module.dealloc(buftxin);
        module.dealloc(buftxid);

        return txin;
};

/**
 * Create a TxOut from the given extended address and amount
 *
 * @param module - the WASM module that is used for crypto operations
 * @param addr - the address to send the given amount to
 * @param amount - the amount to send to the given address
 * @returns {*} - a txout (encoded in cbor)
 */
export const newTxOut = (module, addr, amount) => {
        const bufaddr = newArray(module, addr);
        const buftxout = newArray0(module, 1024);

        let rsz = module.wallet_txout_create(bufaddr, addr.length, amount, buftxout);
        let txout = copyArray(module, buftxout, rsz);

        module.dealloc(buftxout);
        module.dealloc(bufaddr);

        return txout;
};

/**
 * Create an empty Tx
 *
 * @param module - the WASM module that is used for crypto operations
 * @returns {*} - a tx (encoded in cbor)
 */
export const create = (module) => {
        const buftx = newArray0(module, 1024);

        let rsz = module.wallet_tx_new(buftx);
        let tx = copyArray(module, buftx, rsz);

        module.dealloc(buftx);

        return tx;
};

/**
 * Add the given TxIn to the Tx
 *
 * @param module - the WASM module that is used for crypto operations
 * @param tx     - the transaction to add the given TxIn
 * @param txin   - the TxIn to add in the given Tx
 * @returns {*} - a tx (encoded in cbor)
 */
export const addInput = (module, tx, txin) => {
        const buftx = newArray(module, tx);
        const buftxin = newArray(module, txin);

        const bufout = newArray0(module, 1024);

        let rsz = module.wallet_tx_add_txin(buftx, tx.length, buftxin, txin.length, bufout);
        let out = copyArray(module, bufout, rsz);

        module.dealloc(bufout);
        module.dealloc(buftxin);
        module.dealloc(buftx);

        return out;
};

/**
 * Add the given TxOut to the Tx
 *
 * @param module - the WASM module that is used for crypto operations
 * @param tx     - the transaction to add the given TxOut
 * @param txout  - the TxOut to add in the given Tx
 * @returns {*} - a tx (encoded in cbor)
 */
export const addOutput = (module, tx, txout) => {
        const buftx = newArray(module, tx);
        const buftxout = newArray(module, txout);

        const bufout = newArray0(module, 1024);

        let rsz = module.wallet_tx_add_txout(buftx, tx.length, buftxout, txout.length, bufout);
        let out = copyArray(module, bufout, rsz);

        module.dealloc(bufout);
        module.dealloc(buftxout);
        module.dealloc(buftx);

        return out;
};


/**
 * Sign the given tx, this function returns the signature, not the TxInWitness
 *
 * @param module - the WASM module that is used for crypto operations
 * @param tx     - the transaction to add the given TxOut
 * @param xprv   - the extended private key to sign the transaction with
 * @returns {*} - the signature
 */
export const sign = (module, tx, xprv) => {
        const buftx = newArray(module, tx);
        const bufxprv = newArray(module, xprv);
        const bufsig = newArray0(module, 64);

        module.wallet_tx_sign(bufxprv, buftx, tx.length, bufsig);
        let result = copyArray(module, bufsig, 64);

        module.dealloc(bufsig);
        module.dealloc(bufxprv);
        module.dealloc(buftx);

        return result
};

/**
 * Verify the given signature of a tx
 *
 * @param module  - the WASM module that is used for crypto operations
 * @param tx      - the transaction to add the given TxOut
 * @param address - the extended address we are verifying
 * @param xpub    - the extended private key to sign the transaction with
 * @returns {*}   - true or false
 */
export const verify = (module, tx, xpub, signature) => {
        const buftx = newArray(module, tx);
        const bufxpub = newArray(module, xpub);
        const bufsig  = newArray(module, signature);

        let result = module.wallet_tx_verify(bufxpub, buftx, tx.length, bufsig);

        module.dealloc(bufsig);
        module.dealloc(bufxpub);
        module.dealloc(buftx);

        return result === 0
};

export default {
  newTxOut: apply(newTxOut, RustModule),
  newTxIn:  apply(newTxIn, RustModule),
  create: apply(create, RustModule),
  addInput: apply(addInput, RustModule),
  addOutput: apply(addOutput, RustModule),
  sign: apply(sign, RustModule),
  verify: apply(verify, RustModule),
};
