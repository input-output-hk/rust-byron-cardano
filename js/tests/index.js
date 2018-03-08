import { expect } from 'chai';
import { HdWallet } from '../../dist/index.js';

describe('HdWallet', function() {

  beforeEach(function() {
    this.seed = new Uint8Array([
      0xe3, 0x55, 0x24, 0xa5, 0x18, 0x03, 0x4d, 0xdc, 0x11, 0x92, 0xe1, 0xda, 0xcd, 0x32, 0xc1, 0xed, 0x3e,
      0xaa, 0x3c, 0x3b, 0x13, 0x1c, 0x88, 0xed, 0x8e, 0x7e, 0x54, 0xc4, 0x9a, 0x5d, 0x09, 0x98
    ]);
  });

  describe('fromSeed()', function() {

    it('creates a hd wallet from seed', function() {
      return HdWallet.fromSeed(this.seed)
        .then((wallet) => {
          expect(wallet).instanceOf(Uint8Array);
        })
    });
  });

  describe('toPublic()', function() {

    it('creates the public key from given wallet', function() {
      return HdWallet.fromSeed(this.seed)
        .then((wallet) => HdWallet.toPublic(wallet))
        .then((publicKey) => {
          expect(publicKey).instanceOf(Uint8Array);
        });
    });
  });

});
