const expect = require('chai').expect;
const CardanoCrypto = require('../../dist/index.js');

describe('PaperWallet', function() {

  describe('#scramble', function() {

    it('generates 15 words mnenomics from 12 words input', function() {
      const iv = new Uint8Array([0x00, 0x00, 0x00, 0x00]);
      const input = 'legal winner thank year wave sausage worth useful legal winner thank yellow';
      const passphrase = '';

      expect(CardanoCrypto.PaperWallet.scrambleStrings(iv, passphrase, input)).equal(
        'abandon abandon abandon win nest chef want salt join shove minor december miss oak name'
      );
    });
  });

  describe('#unscramble', function() {

    it('retrieves the original 12 words mnenomics from scrambled 15 words input', function() {
      const input = 'abandon abandon abandon win nest chef want salt join shove minor december miss oak name';
      const passphrase = '';

      expect(CardanoCrypto.PaperWallet.unscrambleStrings(passphrase, input)).equal(
        'legal winner thank year wave sausage worth useful legal winner thank yellow'
      );
    });
  });

});
