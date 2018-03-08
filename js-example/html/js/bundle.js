function fetchAndInstantiate(url, importObject) {
  return fetch(url).then(response =>
    response.arrayBuffer()
  ).then(bytes =>
    WebAssembly.instantiate(bytes, importObject)
  ).then(results =>
    results.instance
  );
}

function base16(u8) {
        var b16 = "";
        function pad2(str) {
                return (str.length < 2) ? "0"+str : str;
        }
        for(var x = 0; x < u8.length; x++) {
                b16 += pad2(u8[x].toString(16));
        }
        return b16;
}

// Copy a nul-terminated string from the buffer pointed to.
// Consumes the old data and thus deallocated it.
function copyCStr(module, ptr) {
  let orig_ptr = ptr;
  const collectCString = function* () {
    let memory = new Uint8Array(module.memory.buffer);
    while (memory[ptr] !== 0) {
      if (memory[ptr] === undefined) { throw new Error("Tried to read undef mem") }
      yield memory[ptr]
      ptr += 1
    }
  }

  const buffer_as_u8 = new Uint8Array(collectCString());
  //const utf8Decoder = new TextDecoder("UTF-8");
  const buffer_as_utf8 = base16(buffer_as_u8); // utf8Decoder.decode(buffer_as_u8);
  module.dealloc_str(orig_ptr);
  return buffer_as_utf8
}

function copyArray(module, ptr, sz) {
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

function getStr(module, ptr, len) {
  const getData = function* (ptr, len) {
    let memory = new Uint8Array(module.memory.buffer);
    for (let index = 0; index < len; index++) {
      if (memory[ptr] === undefined) { throw new Error(`Tried to read undef mem at ${ptr}`) }
      yield memory[ptr + index]
    }
  }

  const buffer_as_u8 = new Uint8Array(getData(ptr/8, len/8));
  const utf8Decoder = new TextDecoder("UTF-8");
  const buffer_as_utf8 = utf8Decoder.decode(buffer_as_u8);
  return buffer_as_utf8;
}

function newArray(module, b) {
    let len = b.length
    let ptr = module.alloc(len)

    let memory = new Uint8Array(module.memory.buffer)
    for (let i = 0; i < len; i++) {
        memory[ptr+i] = b[i]
    }
    return ptr
}

function newArray0(module, sz) {
    let ptr = module.alloc(sz)

    let memory = new Uint8Array(module.memory.buffer)
    for (let i = 0; i < sz; i++) {
        memory[ptr+i] = 0
    }
    return ptr
}

function newString(module, str) {
  const utf8Encoder = new TextEncoder("UTF-8");
  let string_buffer = utf8Encoder.encode(str)
  let len = string_buffer.length
  let ptr = module.alloc(len+1)

  let memory = new Uint8Array(module.memory.buffer);
  for (i = 0; i < len; i++) {
    memory[ptr+i] = string_buffer[i]
  }

  memory[ptr+len] = 0;

  return ptr;
}
