export const newArray = (module, b, isZero=false) => {
  let len = b.length
  let ptr = module.alloc(len)

  let memory = new Uint8Array(module.memory.buffer)
  for (let i = 0; i < len; i++) {
    memory[ptr+i] = isZero ? 0 : b[i];
  }
  return ptr
}

export const copyArray = (module, ptr, sz) => {
  const collect = function* () {
    let memory = new Uint8Array(module.memory.buffer);
    let i = 0;
    while (i < sz) {
      yield memory[ptr+i];
      i += 1
    }
  }

  const buffer_as_u8 = new Uint8Array(collect());
  return buffer_as_u8
}
