export const newArray = (module, b, isZero=false) => {
  const len = b.length;
  const ptr = module.alloc(len);

  let memory = new Uint8Array(module.memory.buffer);
  for (let i = 0; i < len; i++) {
    memory[ptr+i] = isZero ? 0 : b[i];
  }
  return ptr
};

export const copyArray = (module, ptr, sz) => {
  const collect = function* () {
    let memory = new Uint8Array(module.memory.buffer);
    let i = 0;
    while (i < sz) {
      yield memory[ptr+i];
      i += 1
    }
  };

  return new Uint8Array(collect());
};
