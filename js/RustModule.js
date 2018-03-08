import loadModule from '../target/wasm32-unknown-unknown/release/wallet_wasm.wasm';
import { applyModule } from './utils/wasm';

let Module = null;

// Ensure we are only creating a single instance of the web assembly module
export const loadRustModule = () => Module ?
  Promise.resolve(Module)
  :
  loadModule().then((module) => {
    Module = module.instance.exports;
    return Module;
  }
);

export default {
  blake2b_256: applyModule(loadRustModule, (module) => module.blake2b_256),
};
