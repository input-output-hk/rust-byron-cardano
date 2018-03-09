import bip39 from 'bip39';
import { Buffer } from 'safe-buffer';
import RustModule from './RustModule';
import Blake2b from './Blake2b';
import { apply } from './utils/functions';

export const mnemonicToSeed = (module, mnemonic) => (
  Blake2b.blake2b_256(bip39.mnemonicToEntropy(mnemonic))
);

export const mnemonicToEntropy = (mnemonic) => (
  Buffer.from(bip39.mnemonicToEntropy(mnemonic), 'hex')
);

export default {
  mnemonicToSeed: apply(mnemonicToSeed, RustModule),
  mnemonicToEntropy,
}
