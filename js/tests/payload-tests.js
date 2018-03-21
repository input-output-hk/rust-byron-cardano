const expect = require('chai').expect;
const CardanoCrypto = require('../../dist/index.js');


const TEST_VECTORS = [
  {
     pubkey: new Uint8Array([28, 12, 58, 225, 130, 94, 144, 182, 221, 218, 63, 64, 161, 34, 192, 7, 225, 0, 142, 131, 178, 225, 2, 193, 66, 186, 239, 183, 33, 215, 44, 26, 93, 54, 97, 222, 185, 6, 79, 45, 14, 3, 254, 133, 214, 128, 112, 178, 254, 51, 180, 145, 96, 89, 101, 142, 40, 172, 127, 127, 145, 202, 75, 18]),
     key: new Uint8Array([136, 134, 222, 55, 140, 182, 153, 42, 33, 114, 29, 200, 219, 74, 28, 233, 127, 123, 104, 2, 129, 9, 77, 187, 142, 3, 229, 30, 127, 183, 201, 179]),
     derivation_path: new Uint32Array([0,1,2]),
     payload: new Uint8Array([229, 123, 216, 139, 186, 31, 136, 170, 141, 206, 193, 201, 206, 53, 33, 116, 160, 227, 158, 62])
  }
];

let mkTest = (i) => {
    const { pubkey, key, derivation_path, payload } = TEST_VECTORS[0];

    describe('Test ' + i, function() {
        it('initialise a payload key', function() {
            expect(CardanoCrypto.Payload.initialise(pubkey))
                .deep.equal(key);
        });

        it('encrypt a path', function() {
            expect(CardanoCrypto.Payload.encrypt_derivation_path(key, derivation_path))
                .deep.equal(payload);
        });

        it('decrypt a payload', function() {
            expect(CardanoCrypto.Payload.decrypt_derivation_path(key, payload))
                .deep.equal(derivation_path);
        });
    });
}

describe('Payload', function() {
    for (let i = 0; i < TEST_VECTORS.length; i++) {
        mkTest(i);
    }
});
