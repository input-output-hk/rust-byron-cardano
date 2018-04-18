const expect = require('chai').expect;
const CardanoCrypto = require('../../dist/index.js');

// Implemented proposal: https://github.com/input-output-hk/cardano-specs/blob/master/proposals/0001-PaperWallet.md
const TEST_VECTORS = [
  {
    iv: new Uint8Array([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
    input: 'legal winner thank year wave sausage worth useful legal winner thank yellow',
    passphrase: '',
    scrambledWords: 'abandon abandon abandon abandon abandon about twelve early chronic curtain ancient judge pond style twin spread asthma enable',
  },
  {
    iv: new Uint8Array([0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]),
    input: 'fold parrot feature figure stay blanket woman grain huge orphan key exile',
    passphrase: 'Cardano Ada',
    scrambledWords: 'abandon amount liar amount expire advance afraid evil author zero dumb elite cover few mirror goat remain vapor'
  },
  {
    iv: new Uint8Array([0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a, 0x2a]),
    input: 'zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong',
    passphrase: 'This is a very long passphrase. This is a very long passphrase. This is a very long passphrase. This is a very long passphrase.',
    scrambledWords: 'clay eyebrow melody february pencil betray build cart insane great coconut champion ancient catch provide horn merit cinnamon'
  }
];

describe('PaperWallet', function() {

  describe('#scramble', function() {

    describe('Test Vector 1: without passphrase', function() {

      it('generates the correct 15 words mnenomics from 12 words input', function() {
        const { iv, input, passphrase, scrambledWords } = TEST_VECTORS[0];
        expect(CardanoCrypto.PaperWallet.scrambleStrings(iv, passphrase, input)).equal(scrambledWords);
      });
    });

    describe('Test Vector 2: short passphrase', function() {

      it('generates the correct 15 words mnenomics from 12 words input', function() {
        const { iv, input, passphrase, scrambledWords } = TEST_VECTORS[1];
        expect(CardanoCrypto.PaperWallet.scrambleStrings(iv, passphrase, input)).equal(scrambledWords);
      });
    });

    describe('Test Vector 3: long passphrase', function() {

      it('generates the correct 15 words mnenomics from 12 words input', function() {
        const { iv, input, passphrase, scrambledWords } = TEST_VECTORS[2];
        expect(CardanoCrypto.PaperWallet.scrambleStrings(iv, passphrase, input)).equal(scrambledWords);
      });
    });
  });

  describe('#unscramble', function() {

    describe('Test Vector 1: without passphrase', function() {

      it('retrieves the original 12 words mnenomics from scrambled 15 words input', function() {
        const { input, passphrase, scrambledWords } = TEST_VECTORS[0];
        expect(CardanoCrypto.PaperWallet.unscrambleStrings(passphrase, scrambledWords)).equal(input);
      });
    });

    describe('Test Vector 2: short passphrase', function() {

      it('retrieves the original 12 words mnenomics from scrambled 15 words input', function() {
        const { input, passphrase, scrambledWords } = TEST_VECTORS[1];
        expect(CardanoCrypto.PaperWallet.unscrambleStrings(passphrase, scrambledWords)).equal(input);
      });
    });

    describe('Test Vector 3: long passphrase', function() {

      it('retrieves the original 12 words mnenomics from scrambled 15 words input', function() {
        const { input, passphrase, scrambledWords } = TEST_VECTORS[2];
        expect(CardanoCrypto.PaperWallet.unscrambleStrings(passphrase, scrambledWords)).equal(input);
      });
    });
  });
});
