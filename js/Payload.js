import RustModule from './RustModule';
import { newArray, newArray0, copyArray, newArrayU32, newArrayU32_0, copyArrayU32 } from './utils/arrays';
import { apply } from './utils/functions';


/**
 * Function to create an Payload Key. This can then be used to encrypt
 * or decrypt a derivation path.
 *
 * @param module - the WASM module that is used for crypto operations
 * @param xpub - a public key
 * @returns {*} - a HDPkey that can be used to encrypt or decrypt a
 *                derivation_path
 */
export const initialise = (module, xpub) => {
  const bufxpub  = newArray(module, xpub);
  const bufhdkey = newArray0(module, 32);

  module.wallet_payload_initiate(bufxpub, bufhdkey);
  let key = copyArray(module, bufhdkey, 32);

  module.dealloc(bufxpub);
  module.dealloc(bufhdkey);
  return key;
};

/**
 * Encrypt the given derivation path (an array of unsigned 32 bit integer).
 *
 * @param module - the WASM module that is used for crypto operations
 * @param key - the encryption key initialised with `initialise`
 * @param derivation_path - the derivation path to encrypt
 * @returns {*} - encrypted derivation path
 */
export const encrypt_derivation_path = (module, key, derivation_path) => {
  const bufhdkey = newArray(module, key);
  const bufpath  = newArrayU32(module, derivation_path);
  const bufenc   = newArray0(module, 1024);

  let rsz = module.wallet_payload_encrypt(bufhdkey, bufpath, derivation_path.length, bufenc);
  let enc = copyArray(module, bufenc, rsz);

  module.dealloc(bufhdkey);
  module.dealloc(bufpath);
  module.dealloc(bufenc);

  return enc;
}

/**
 * Decrypt the given payload into a derivation path (an array of unsigned 32 bit integer).
 *
 * @param module - the WASM module that is used for crypto operations
 * @param key - the encryption key initialised with `initialise`
 * @param payload - the derivation path to decrypt
 * @returns {*} - null or a decrypted derivation path
 */
export const decrypt_derivation_path = (module, key, payload) => {
  const bufhdkey   = newArray(module, key);
  const bufpayload = newArray(module, payload);
  const bufdec     = newArrayU32_0(module, 64);

  let dec = null;

  let rsz = module.wallet_payload_decrypt(bufhdkey, bufpayload, payload.length, bufdec);
  if (rsz !== -1) {
      dec = copyArrayU32(module, bufdec, rsz);
  }

  module.dealloc(bufhdkey);
  module.dealloc(bufpayload);
  module.dealloc(bufdec);

  return dec;
}

export default {
  initialise: apply(initialise, RustModule),
  encrypt_derivation_path: apply(encrypt_derivation_path, RustModule),
  decrypt_derivation_path: apply(decrypt_derivation_path, RustModule)
}
