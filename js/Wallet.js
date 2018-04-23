import iconv from 'iconv-lite';
import RustModule from './RustModule';
import { newArray, newArray0, copyArray } from './utils/arrays';
import { apply } from './utils/functions';
import { base16 } from './utils/strings';

/**
 * Create a wallet object from the given seed.
 *
 * @param module - the WASM module that is used for crypto operations
 * @param seed   - the 32 bytes seed to generate the wallet from
 * @returns {*}  - a wallet object (JSON object)
 */
export const fromSeed = (module, seed) => {
    const input_str = JSON.stringify(seed);
    const input_array = iconv.encode(input_str, 'utf8');

    const bufinput  = newArray(module, input_array);
    const bufoutput = newArray0(module, 4096);

    let rsz = module.xwallet_create(bufinput, input_array.length, bufoutput);
    let output_array = copyArray(module, bufoutput, rsz);

    module.dealloc(bufoutput);
    module.dealloc(bufinput);

    let output_str = iconv.decode(Buffer.from(output_array), 'utf8');
    return JSON.parse(output_str);
};

/**
 * Generate addresses for the given wallet.
 *
 * @example
 * ```
 * const wallet    = CardanoCrypto.Wallet.fromSeed([0,1,2,3,4..]).result;
 * const addresses = CardanoCrypto.Wallet.generateAddresses(0, "External", [0,1,2,3,4]).result;
 * ```
 *
 * @param module - the WASM module that is used for crypto operations
 * @param wallet - The wallet object as created by the `fromSeed` function
 * @param account - the account number
 * @param type    - the addresses type ("Internal" or "External")
 * @param indices - the addresse indices
 * @returns {*}  - a list of ready to use addresses
 */
export const generateAddresses = (module, wallet, account, type, indices) => {
    const input = { wallet: wallet
                  , account: account
                  , address_type: type
                  , indices: indices
                  };
    const input_str = JSON.stringify(input);
    const input_array = iconv.encode(input_str, 'utf8');

    const bufinput  = newArray(module, input_array);
    const bufoutput = newArray0(module, 4096);

    let rsz = module.xwallet_addresses(bufinput, input_array.length, bufoutput);
    let output_array = copyArray(module, bufoutput, rsz);

    module.dealloc(bufoutput);
    module.dealloc(bufinput);

    let output_str = iconv.decode(Buffer.from(output_array), 'utf8');
    return JSON.parse(output_str);
};

/**
 * Generate a ready to send, signed, transaction.
 *
 * @example
 * ```
 * let seed = 'compute seed from mnemonic';
 * let wallet = CardanoCrypto.Wallet.fromSeed(seed).result;
 *
 * // the inputs are the resolved UTxO
 * //
 * // they contained the TxId and the index of where the TxOut is and the content of the
 * // TxOut.
 * let inputs =
 *     [ { ptr: { index: 42
 *              , id: "1c7b178c1655628ca87c7da6a5d9d13c1e0a304094ac88770768d565e3d20e0b"
 *              }
 *       , value: { address: "DdzFFzCqrhtCUjHyzgvgigwA5soBgDxpc8WfnG1RGhrsRrWMV8uKdpgVfCXGgNuXhdN4qxPMvRUtbUnWhPzxSdxJrWzPqACZeh6scCH5"
 *                , value: 92837348
 *                }
 *       , addressing: { account: 0, change: 0, index: 9 }
 *       }
 *     ];
 *
 * // where we want to send money to
 * let outputs =
 *     [ { address: "DdzFFzCqrhtCUjHyzgvgigwA5soBgDxpc8WfnG1RGhrsRrWMV8uKdpgVfCXGgNuXhdN4qxPMvRUtbUnWhPzxSdxJrWzPqACZeh6scCH5"
 *       , value: 666
 *       }
 *     ];
 *
 * // where the fee will need to be sent
 * let fee_addr = "DdzFFzCqrhtCUjHyzgvgigwA5soBgDxpc8WfnG1RGhrsRrWMV8uKdpgVfCXGgNuXhdN4qxPMvRUtbUnWhPzxSdxJrWzPqACZeh6scCH5";
 * let change_addr = "DdzFFzCqrhtCUjHyzgvgigwA5soBgDxpc8WfnG1RGhrsRrWMV8uKdpgVfCXGgNuXhdN4qxPMvRUtbUnWhPzxSdxJrWzPqACZeh6scCH5";
 *
 * let signed_tx = CardanoCrypto.Wallet.spend(wallet, inputs, outputs, fee_addr, change_addr).result;
 * ```
 *
 * @param module - the WASM module that is used for crypto operations
 * @param wallet - The wallet object as created by the `fromSeed` function
 * @param inputs - the list of inputs
 * @param outputs - the list of payment to make
 * @param fee_addr - the address to send the fee to
 * @param change_addr - the address to send the change to
 * @returns {*}  - a ready to use, signed transaction encoded in cbor
 */
export const spend = (module, wallet, inputs, outputs, fee_addr, change_addr) => {
    const input = { wallet: wallet
                  , inputs: inputs
                  , outputs: outputs
                  , fee_addr: fee_addr
                  , change_addr: change_addr
                  };
    const input_str = JSON.stringify(input);
    const input_array = iconv.encode(input_str, 'utf8');

    const bufinput  = newArray(module, input_array);
    const bufoutput = newArray0(module, 4096);

    let rsz = module.xwallet_spend(bufinput, input_array.length, bufoutput);
    let output_array = copyArray(module, bufoutput, rsz);

    module.dealloc(bufoutput);
    module.dealloc(bufinput);

    let output_str = iconv.decode(Buffer.from(output_array), 'utf8');
    return JSON.parse(output_str);
};

export default {
  fromSeed: apply(fromSeed, RustModule),
  generateAddresses: apply(generateAddresses, RustModule),
  spend: apply(spend, RustModule)
};
