export const newArray = (module, b) => {
  const len = b.length;
  const ptr = module.alloc(len);

  let memory = new Uint8Array(module.memory.buffer);
  for (let i = 0; i < len; i++) {
    memory[ptr+i] = b[i];
  }
  return ptr
};

export const newArray0 = (module, len) => {
  let ptr = module.alloc(len)

  let memory = new Uint8Array(module.memory.buffer);
  for (let i = 0; i < len; i++) {
    memory[ptr+i] = 0;
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

// create an array of 32bit integer
export const newArrayU32 = (module, b) => {
    let ptr = module.alloc(b.length * 4)
    let memory = new Uint8Array(module.memory.buffer);
    for (let i = 0; i < b.length; i++) {
        memory[ptr + i * 4 + 0] = (b[i] >>  0) & 0xFF;
        memory[ptr + i * 4 + 1] = (b[i] >>  8) & 0xFF;
        memory[ptr + i * 4 + 2] = (b[i] >> 16) & 0xFF;
        memory[ptr + i * 4 + 3] = (b[i] >> 24) & 0xFF;
    }
    return ptr;
};


export const newArrayU32_0 = (module, len) => {
    return newArray0(module, len * 4);
};

// copy array of 32bits element
export const copyArrayU32 = (module, ptr, sz) => {
    const collect = function* () {
        let memory = new Uint8Array(module.memory.buffer);
        for (let i = 0; i < sz; i++) {
            let b = 0;
            b |= (memory[ptr + i * 4 + 0] <<  0);
            b |= (memory[ptr + i * 4 + 1] <<  8);
            b |= (memory[ptr + i * 4 + 2] << 16);
            b |= (memory[ptr + i * 4 + 3] << 24);
            yield b;
        }
    };

    return new Uint32Array(collect());
};
