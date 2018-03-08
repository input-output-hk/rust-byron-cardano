"use strict";

var bip39 = require('bip39');

exports.mnemonicToSeedImpl = function (m) {
    try {
        // TODO: there is a hack in that function that is not upstream
        var e = bip39.mnemonicToEntropy(m);
        return window.CardanoCrypto.Blake2b.blake2b_256(e);
    } catch(e) {
        console.error("BIP39 mnemonicToSeed error:", e);
        return null;
    }
};

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
exports.seedToBase64Impl = function (seed) {
    try {
        return base16(seed);
    } catch (e) {
        console.error("BIP39 seedToHexImpl error:", e);
        return "";
    }
};
