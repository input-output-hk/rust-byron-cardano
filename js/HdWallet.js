import RustModule from './RustModule';
import { newArray, newArray0, copyArray } from './utils/arrays';
import { apply } from './utils/functions';

export const fromSeed = (module, seed) => {
  const bufseed = newArray(module, seed);
  const bufxprv = newArray0(module, 96);
  module.wallet_from_seed(bufseed, bufxprv);
  let result = copyArray(module, bufxprv, 96);
  module.dealloc(bufseed);
  module.dealloc(bufxprv);
  return result;
};

export const toPublic = (module, xprv) => {
  const bufxprv = newArray(module, xprv);
  const bufxpub = newArray0(module, 64);
  module.wallet_to_public(bufxprv, bufxpub);
  let result = copyArray(module, bufxpub, 64);
  module.dealloc(bufxprv);
  module.dealloc(bufxpub);
  return result;
};

export const derivePrivate = (module, xprv, index) => {
  const bufxprv = newArray(module, xprv);
  const bufchild = newArray0(module, xprv.length);
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
  const bufchild = newArray0(module, xpub.length);
  const r = module.wallet_derive_public(bufxpub, index, bufchild);
  const result = copyArray(module, bufchild, xpub.length);
  module.dealloc(bufxpub);
  module.dealloc(bufchild);
  return result
};

export const sign = (module, xprv, msg) => {
  let length = msg.length;
  const bufsig = newArray0(module, 64);
  const bufxprv = newArray(module, xprv);
  const bufmsg = newArray(module, msg);
  module.wallet_sign(bufxprv, bufmsg, length, bufsig);
  let result = copyArray(module, bufsig, 64);
  module.dealloc(bufxprv);
  module.dealloc(bufmsg);
  module.dealloc(bufsig);
  return result
};

export const publicKeyToAddress = (module, xpub, payload) => {
  const bufxpub    = newArray(module, xpub);
  const bufpayload = newArray(module, payload);
  const bufaddr    = newArray0(module, 1024);

  let rs = module.wallet_public_to_address(bufxpub, bufpayload, payload.length, bufaddr);
  let addr = copyArray(module, bufaddr, rs);

  module.dealloc(bufaddr);
  module.dealloc(bufpayload);
  module.dealloc(bufxpub);

  return addr;
};


export default {
  fromSeed: apply(fromSeed, RustModule),
  toPublic: apply(toPublic, RustModule),
  derivePrivate: apply(derivePrivate, RustModule),
  derivePublic: apply(derivePublic, RustModule),
  sign: apply(sign, RustModule),
  publicKeyToAddress: apply(publicKeyToAddress, RustModule),
};
