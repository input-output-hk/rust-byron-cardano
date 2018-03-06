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

export const derivePrivate = (module, xprv, index) => {
  const bufxprv = newArray(module, xprv);
  const bufchild = newArray(module, xprv.length, true);
  module.wallet_derive_private(bufxprv, index, bufchild);
  let result = copyArray(module, bufchild, xprv.length);
  module.dealloc(bufxprv);
  module.dealloc(bufchild);
  return result;
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
  derivePrivate: applyModule(loadRustModule, derivePrivate),
  sign: applyModule(loadRustModule, sign),
};
