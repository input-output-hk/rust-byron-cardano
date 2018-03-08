var Module = {};

fetch("wasm/cardano.wasm").then(response =>
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

    Module.blake2b_256  = mod.exports.blake2b_256;

    Module.wallet_from_seed = mod.exports.wallet_from_seed;
    Module.wallet_to_public = mod.exports.wallet_to_public;
    Module.wallet_derive_private = mod.exports.wallet_derive_private;
    Module.wallet_sign = mod.exports.wallet_sign;

    Module.Pbkdf2 = { sha256: function(password, salt, iters, output_size) {
        let buf_pass = newString(Module, password);
        let buf_salt = newString(Module, salt);
        let outptr = Module.pbkdf2_sha256(buf_pass, buf_salt, iters, output_size);
        let result = copyCStr(Module, outptr);
        Module.dealloc_str(buf_pass);
        Module.dealloc_str(buf_salt);
        return result;
    }};

    Module.Blake2b = { blake2b_256: function (message) {
        let input = newArray(Module, message);
        let output = newArray0(Module, 32);
        Module.blake2b_256(input, message.length, output);
        let result = copyArray(Module, output, 32);
        Module.dealloc(input);
        Module.dealloc(output);
        return result
    }};

    Module.PaperWallet = {
        scramble: function(iv, password, input) {
            if (iv.length != 4) {
                throw new Error('IV must be 4 bytes');
            }
            bufiv = newArray(Module, iv);
            bufinput = newArray(Module, input);
            bufpassword = newArray(Module, password);
            bufoutput = newArray0(Module, input.length + 4);
            Module.paper_scramble(bufiv, bufpassword, password.length, bufinput, input.length, bufoutput);
            let result = copyArray(Module, bufoutput, input.length + 4);
            Module.dealloc(bufiv);
            Module.dealloc(bufinput);
            Module.dealloc(bufpassword);
            Module.dealloc(bufoutput);
            return result;
        },
        unscramble: function(password, input) {
            if (input.length < 4) {
                throw new Error('input must be at least 4 bytes');
            }
            bufinput = newArray(Module, input);
            bufpassword = newArray(Module, password);
            bufoutput = newArray0(Module, input.length - 4);
            Module.paper_unscramble(bufpassword, password.length, bufinput, input.length, bufoutput);
            let result = copyArray(Module, bufxprv, input.length - 4);
            Module.dealloc(bufinput);
            Module.dealloc(bufpassword);
            Module.dealloc(bufoutput);
            return result;
        },
    };

    Module.HdWallet = {
        from_seed: function(seed) {
            bufseed = newArray(Module, seed);
            bufxprv = newArray0(Module, 96);
            Module.wallet_from_seed(bufseed, bufxprv);
            let result = copyArray(Module, bufxprv, 96);
            Module.dealloc(bufseed);
            Module.dealloc(bufxprv);
            return result
        },
        to_public: function(xprv) {
            bufxprv = newArray(Module, xprv);
            bufxpub = newArray0(Module, 64);
            Module.wallet_to_public(bufxprv, bufxpub);
            let result = copyArray(Module, bufxpub, 64);
            Module.dealloc(bufxprv);
            Module.dealloc(bufxpub);
            return result
        },
        derive_private: function(xprv, index) {
            bufxprv = newArray(Module, xprv);
            bufchild = newArray0(Module, xprv.length);
            Module.wallet_derive_private(bufxprv, index, bufchild);
            let result = copyArray(Module, bufchild, xprv.length);
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
            let result = copyArray(Module, bufchild, xpub.length);
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
            let result = copyArray(Module, bufsig, 64);
            Module.dealloc(bufxprv);
            Module.dealloc(bufmsg);
            Module.dealloc(bufsig);
            return result
        }
    };
});
