"use strict";

var randomBytes = require('randombytes');

const utf8Encoder = new TextEncoder("UTF-8");


exports.seedToRootKeyImpl = function (seed) {
    try {
        return window.Module.HdWallet.from_seed (seed);
    } catch (e) {
        console.error("error in seedToRootKey: ", e);
        return null;
    }
};

exports.xprvToXPubImpl = function (xprv) {
    try {
        return window.Module.HdWallet.to_public (xprv);
    } catch (e) {
        console.error("error in xprvToXPub: ", e);
        return null;
    }
};

exports.signImpl = function (xprv) {
    return function (msg) {
        try {
            var string_buffer = utf8Encoder.encode(msg);
            return window.Module.HdWallet.sign (xprv, string_buffer);
            return s;
        } catch (e) {
            console.error("error in signImpl: ", e);
            return null;
        }
    };
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

exports.scrambleImpl = function (entropy) {
    return function (pass) {
        var pass_buffer = utf8Encoder.encode(pass);
        var iv = randomBytes(4);
        try {
            return window.Module.PaperWallet.scramble (iv, pass_buffer, entropy);
        } catch (e) {
            console.error(e);
            return null;
        }
    };
};

exports.showPrivKey = function (xprv) {
    return base16(xprv);
};
exports.showPubKey = function (xpub) {
    return base16(xpub);
};
exports.showSignature = function (sign) {
    return base16(sign);
};
