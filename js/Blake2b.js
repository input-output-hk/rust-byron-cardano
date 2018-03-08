import RustModule from './RustModule';
import { copyArray, newArray, newArray0 } from './utils/arrays';
import { apply } from './utils/functions';

export const blake2b_256 = (module, message) => {
  let input = newArray(module, message);
  let output = newArray0(module, 32);
  module.blake2b_256(input, message.length, output);
  let result = copyArray(module, output, 32);
  module.dealloc(input);
  module.dealloc(output);
  return result
};

export default {
  blake2b_256: apply(blake2b_256, RustModule)
}
