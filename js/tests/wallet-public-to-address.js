const expect = require('chai').expect;
const CardanoCrypto = require('../../dist/index.js');


const TEST_VECTORS = [
  {
     pubkey: new Uint8Array([28, 12, 58, 225, 130, 94, 144, 182, 221, 218, 63, 64, 161, 34, 192, 7, 225, 0, 142, 131, 178, 225, 2, 193, 66, 186, 239, 183, 33, 215, 44, 26, 93, 54, 97, 222, 185, 6, 79, 45, 14, 3, 254, 133, 214, 128, 112, 178, 254, 51, 180, 145, 96, 89, 101, 142, 40, 172, 127, 127, 145, 202, 75, 18]),
     payload: new Uint8Array([229, 123, 216, 139, 186, 31, 136, 170, 141, 206, 193, 201, 206, 53, 33, 116, 160, 227, 158, 62]),
     address: new Uint8Array([130,216,24,88,91,131,88,28,103,191,45,65,229,171,153,52,143,145,167,242,161,192,220,9,182,30,93,8,234,101,164,18,30,9,182,164,162,0,88,32,130,0,88,28,166,217,174,244,117,243,65,137,103,232,127,126,147,242,15,153,216,199,175,64,108,186,20,106,255,219,113,145,1,85,84,229,123,216,139,186,31,136,170,141,206,193,201,206,53,33,116,160,227,158,62,0,26,31,155,182,203])
  }
];

let mkTest = (i) => {
    const { pubkey, payload, address } = TEST_VECTORS[0];

    describe('Test ' + i, function() {
        it('create an address', function() {
            expect(CardanoCrypto.HdWallet.publicKeyToAddress(pubkey, payload))
                .deep.equal(address);
        });
    });
}

describe('WalletPublicToAddress', function() {
    for (let i = 0; i < TEST_VECTORS.length; i++) {
        mkTest(i);
    }
});
