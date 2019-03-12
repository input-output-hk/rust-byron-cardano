#include "../cardano.h"
#include "unity/unity.h"

void test_generate_entropy_from_mnemonics(void) {
    static const char *mnemonics =  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    cardano_entropy entropy;
    uint32_t bytes;
    cardano_bip39_error_t error = cardano_entropy_from_english_mnemonics(mnemonics, &entropy, &bytes);

    uint8_t expected[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    TEST_ASSERT_EQUAL_HEX8_ARRAY(expected, entropy, 16);

    cardano_delete_entropy_array(entropy, bytes);
}

void test_generate_entropy_from_mnemonics_error_code_invalid_word(void) {
    static const char *mnemonics =  "notaword abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    cardano_entropy entropy;
    uint32_t bytes;
    cardano_bip39_error_t error = cardano_entropy_from_english_mnemonics(mnemonics, &entropy, &bytes);

    TEST_ASSERT_EQUAL_HEX32(BIP39_INVALID_MNEMONIC, error);
}

void test_generate_entropy_from_mnemonics_invalid_checksum(void) {
    static const char *mnemonics =  "about abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    cardano_entropy entropy;
    uint32_t bytes;
    cardano_bip39_error_t error = cardano_entropy_from_english_mnemonics(mnemonics, &entropy, &bytes);

    TEST_ASSERT_EQUAL_HEX32(BIP39_INVALID_CHECKSUM, error);
}

uint8_t gen() {
    return 1;
}

void test_generate_entropy_from_random_generator(void) {
    const uint8_t NUMBER_OF_WORDS = 12; 
    cardano_entropy entropy;
    uint32_t bytes;
    cardano_bip39_error_t error = cardano_entropy_from_random(NUMBER_OF_WORDS, gen, &entropy, &bytes);

    uint8_t expected[16] = {1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1};

    TEST_ASSERT_EQUAL(16, bytes);
    TEST_ASSERT_EQUAL_HEX8_ARRAY(expected, entropy, 16);

    cardano_delete_entropy_array(entropy, bytes);
}

void test_generate_entropy_from_random_generator_word_count_error(void) {
    const uint8_t NUMBER_OF_WORDS = 13; 
    cardano_entropy entropy;
    uint32_t bytes;
    cardano_bip39_error_t error = cardano_entropy_from_random(NUMBER_OF_WORDS, gen, &entropy, &bytes);

    TEST_ASSERT_EQUAL_HEX32(BIP39_INVALID_WORD_COUNT, error);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_generate_entropy_from_mnemonics);
    RUN_TEST(test_generate_entropy_from_mnemonics_error_code_invalid_word);
    RUN_TEST(test_generate_entropy_from_mnemonics_invalid_checksum);
    RUN_TEST(test_generate_entropy_from_random_generator);
    RUN_TEST(test_generate_entropy_from_random_generator_word_count_error);
    return UNITY_END();
}