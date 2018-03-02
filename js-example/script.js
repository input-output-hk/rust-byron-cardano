var Module = {};

fetch("cardano.wasm").then(response =>
    response.arrayBuffer()
).then(bytes =>
    WebAssembly.instantiate(bytes, { env: {} })
).then(results => {
    let mod = results.instance;
    Module.alloc   = mod.exports.alloc;
    Module.dealloc = mod.exports.dealloc;
    Module.dealloc_str = mod.exports.dealloc_str;
    Module.pbkdf2_sha256  = mod.exports.pbkdf2_sha256;
    Module.memory  = mod.exports.memory;

    Module.wallet_from_seed = mod.exports.wallet_from_seed;
    Module.wallet_derive_private = mod.exports.wallet_derive_private;
    Module.wallet_sign = mod.exports.wallet_sign;

    var Pbkdf2 = { sha256: function(password, salt, iters, output_size) {
        let buf_pass = newString(Module, password);
        let buf_salt = newString(Module, salt);
        let outptr = Module.pbkdf2_sha256(buf_pass, buf_salt, iters, output_size);
        let result = copyCStr(Module, outptr);
        Module.dealloc_str(buf_pass);
        Module.dealloc_str(buf_salt);
        return result;
    }};

    var PaperWallet = {
        scramble: function(iv, password, data) {
        },
        unscramble: function(password, shielded_data) {
        },
    };

    var HdWallet = {
        from_seed: function(seed) {
            bufseed = newArray(Module, seed);
            bufxprv = newArray0(Module, 96);
            Module.wallet_from_seed(bufseed, bufxprv);
            let result = copy_array(Module, bufxprv, 96);
            Module.dealloc(bufseed);
            Module.dealloc(bufxprv);
            return result
        },
        to_public: function(xprv) {
            bufxprv = newArray(Module, xprv);
            bufxpub = newArray0(Module, 64);
            Module.wallet_to_public(bufxprv, bufxpub);
            let result = copy_array(Module, bufxpub, 64);
            Module.dealloc(bufxprv);
            Module.dealloc(bufxpub);
            return result
        },
        derive_private: function(xprv, index) {
            bufxprv = newArray(Module, xprv);
            bufchild = newArray0(Module, xprv.length);
            Module.wallet_derive_private(bufxprv, index, bufchild);
            let result = copy_array(Module, bufchild, xprv.length);
            Module.dealloc(bufxprv);
            Module.dealloc(bufchild);
            return result
        },
        derive_public: function(xpub, index) {
            if (index >= 0x80000000) {
                throw new Error('cannot do public derivation with hard index');
            }
            bufxpub = newArray(Module, xpub);
            bufchild = newArray0(Module, xpub.length);
            let r = Module.wallet_derive_public(bufxpub, index, bufchild);
            let result = copy_array(Module, bufchild, xpub.length);
            Module.dealloc(bufxpub);
            Module.dealloc(bufchild);
            return result
        },
        sign: function(xprv, msg) {
            let length = msg.length;
            bufsig = newArray0(Module, 64);
            bufxprv = newArray(Module, xprv);
            bufmsg = newArray(Module, msg);
            Module.wallet_sign(bufxprv, bufmsg, length, bufsig);
            let result = copy_array(Module, bufsig, 64);
            Module.dealloc(bufxprv);
            Module.dealloc(bufmsg);
            Module.dealloc(bufsig);
            return result
        }
    };

    var seed = new Uint8Array([0xe3, 0x55, 0x24, 0xa5, 0x18, 0x03, 0x4d, 0xdc, 0x11, 0x92, 0xe1, 0xda, 0xcd, 0x32, 0xc1, 0xed, 0x3e, 0xaa, 0x3c, 0x3b, 0x13, 0x1c, 0x88, 0xed, 0x8e, 0x7e, 0x54, 0xc4, 0x9a, 0x5d, 0x09, 0x98]);

    let v = HdWallet.from_seed(seed);
    let c = HdWallet.derive_private(v, 0x80000000);

    const utf8Encoder = new TextEncoder("UTF-8");
    let string_buffer = utf8Encoder.encode("Hello World");

    let sig = HdWallet.sign(c, string_buffer);
});
