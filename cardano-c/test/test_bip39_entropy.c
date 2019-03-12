#include "../cardano.h"
#include "unity/unity.h"

void test_generate_entropy_from_mnemonics(void) {
    static const char *mnemonics =  "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    cardano_entropy entropy;
    uint32_t bytes;
    int error = cardano_entropy_from_mnemonics(mnemonics, &entropy, &bytes);

    uint8_t expected[16] = {0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    TEST_ASSERT_EQUAL_HEX8_ARRAY(expected, entropy, 16);

    cardano_delete_entropy_array(entropy, bytes);
}

void test_generate_entropy_from_mnemonics_error_code_invalid_word(void) {
    static const char *mnemonics =  "termo abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    cardano_entropy entropy;
    uint32_t bytes;
    int error = cardano_entropy_from_mnemonics(mnemonics, &entropy, &bytes);

    TEST_ASSERT_EQUAL_HEX32(INVALID_MNEMONIC, error);
}

void test_generate_entropy_from_mnemonics_invalid_checksum(void) {
    static const char *mnemonics =  "about abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    cardano_entropy entropy;
    uint32_t bytes;
    int error = cardano_entropy_from_mnemonics(mnemonics, &entropy, &bytes);

    TEST_ASSERT_EQUAL_HEX32(INVALID_CHECKSUM, error);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_generate_entropy_from_mnemonics);
    RUN_TEST(test_generate_entropy_from_mnemonics_error_code_invalid_word);
    RUN_TEST(test_generate_entropy_from_mnemonics_invalid_checksum);
    return UNITY_END();
}