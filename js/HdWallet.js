import { newArray, copyArray } from './utils/arrays';
import { applyModule } from './utils/wasm';
import { loadRustModule } from './RustModule';

export const fromSeed = (module, seed) => {
  const bufseed = newArray(module, seed);
  const bufxprv = newArray(module, 96, true);
  module.wallet_from_seed(bufseed, bufxprv);
  let result = copyArray(module, bufxprv, 96);
  module.dealloc(bufseed);
  module.dealloc(bufxprv);
  return result;
};

export const toPublic = (module, xprv) => {
  const bufxprv = newArray(module, xprv);
  const bufxpub = newArray(module, 64, true);
  module.wallet_to_public(bufxprv, bufxpub);
  let result = copyArray(module, bufxpub, 64);
  module.dealloc(bufxprv);
  module.dealloc(bufxpub);
  return result;
};

export const derivePrivate = (module, xprv, index) => {
  const bufxprv = newArray(module, xprv);
  const bufchild = newArray(module, xprv.length, true);
  module.wallet_derive_private(bufxprv, index, bufchild);
  let result = copyArray(module, bufchild, xprv.length);
  module.dealloc(bufxprv);
  module.dealloc(bufchild);
  return result;
};

export const derivePublic = (module, xpub, index) => {
  if (index >= 0x80000000) {
    throw new Error('cannot do public derivation with hard index');
  }
  const bufxpub = newArray(module, xpub);
  const bufchild = newArray(module, xpub.length, true);
  const r = module.wallet_derive_public(bufxpub, index, bufchild);
  const result = copyArray(module, bufchild, xpub.length);
  module.dealloc(bufxpub);
  module.dealloc(bufchild);
  return result
};

export const sign = (module, xprv, msg) => {
  let length = msg.length;
  const bufsig = newArray(module, 64, true);
  const bufxprv = newArray(module, xprv);
  const bufmsg = newArray(module, msg);
  module.wallet_sign(bufxprv, bufmsg, length, bufsig);
  let result = copyArray(module, bufsig, 64);
  module.dealloc(bufxprv);
  module.dealloc(bufmsg);
  module.dealloc(bufsig);
  return result;
};

export default {
  fromSeed: applyModule(loadRustModule, fromSeed),
  toPublic: applyModule(loadRustModule, toPublic),
  derivePrivate: applyModule(loadRustModule, derivePrivate),
  derivePublic: applyModule(loadRustModule, derivePublic),
  sign: applyModule(loadRustModule, sign),
};
