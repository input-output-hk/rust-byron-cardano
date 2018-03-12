import bip39 from 'bip39';
import { Buffer } from 'safe-buffer';
import RustModule from './RustModule';
import Blake2b from './Blake2b';
import { apply } from './utils/functions';

export const mnemonicToEntropy = (mnemonic) => (
  Buffer.from(bip39.mnemonicToEntropy(mnemonic), 'hex')
);

export const entropyToMnenomic = (entropy) => (
  bip39.entropyToMnemonic(entropy)
);

export default {
  mnemonicToEntropy,
  entropyToMnenomic,
}
