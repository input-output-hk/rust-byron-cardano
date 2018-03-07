import { newArray, newArray0, copyArray } from './utils/arrays';
import { applyModule } from './utils/wasm';
import { loadRustModule } from './RustModule';

export const scramble = (module, iv, password, input) => {
    if (iv.length != 4) {
        throw new Error('IV must be 4 bytes');
    }
    const bufiv = newArray(module, iv);
    const bufinput = newArray(module, input);
    const bufpassword = newArray(module, password);
    const bufoutput = newArray0(module, input.length + 4);
    module.paper_scramble(bufiv, bufpassword, password.length, bufinput, input.length, bufoutput);
    let result = copy_array(module, bufoutput, input.length + 4);
    module.dealloc(bufiv);
    module.dealloc(bufinput);
    module.dealloc(bufpassword);
    module.dealloc(bufoutput);
    return result;
};

export const unscramble = (module, password, input) => {
    if (input.length < 4) {
        throw new Error('input must be at least 4 bytes');
    }
    const bufinput = newArray(module, input);
    const bufpassword = newArray(module, password);
    const bufoutput = newArray0(module, input.length - 4);
    module.paper_unscramble(bufpassword, password.length, bufinput, input.length, bufoutput);
    let result = copy_array(module, bufxprv, input.length - 4);
    module.dealloc(bufinput);
    module.dealloc(bufpassword);
    module.dealloc(bufoutput);
    return result;
};

export default {
  scramble: applyModule(loadRustModule, scramble),
  unscramble: applyModule(loadRustModule, unscramble),
}
