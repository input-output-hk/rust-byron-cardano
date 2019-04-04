#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "../cardano.h"
#include "unity/unity.h"

static const uint8_t static_wallet_entropy[16] = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15};

void test_can_create_address(void)
{
    static const char *alias = "Test Wallet";
    static char *address[1];
    size_t NUMBER_OF_ADDRESSES = sizeof(address) / sizeof(char *);

    cardano_wallet *wallet;
    cardano_result wallet_rc = cardano_wallet_new(
        static_wallet_entropy,
        sizeof(static_wallet_entropy),
        "abc",
        strlen("abc"),
        &wallet);

    TEST_ASSERT_EQUAL_MESSAGE(0, wallet_rc, "The wallet creation failed");

    cardano_account *account = cardano_account_create(wallet, alias, 0);

    TEST_ASSERT_MESSAGE(account, "The account creation failed");

    cardano_account_generate_addresses(account, 0, 0, NUMBER_OF_ADDRESSES, address);

    TEST_ASSERT_MESSAGE(!cardano_address_is_valid(address[0]), "The generated address is invalid");

    cardano_account_delete_addresses(address, NUMBER_OF_ADDRESSES);

    cardano_account_delete(account);

    cardano_wallet_delete(wallet);
}

void invalid_entropy_size_returns_failure()
{
    char *password = "abc";
    const uint8_t invalid_wallet_entropy[4] = {0};

    cardano_wallet *wallet;
    cardano_result wallet_rc = cardano_wallet_new(
        invalid_wallet_entropy, sizeof(invalid_wallet_entropy), password, strlen(password), &wallet);

    TEST_ASSERT_EQUAL(1, wallet_rc);
}

void valid_entropy_size_returns_success()
{
    const size_t valid_sizes[] = {12, 16, 20, 24, 28, 32};
    const char *password = "abc";
    for (int i = 0; i < sizeof(valid_sizes) / sizeof(size_t); ++i)
    {
        size_t size = valid_sizes[i];
        uint8_t *valid_wallet_entropy = malloc(size);
        cardano_wallet *wallet;
        cardano_result wallet_rc = cardano_wallet_new(
            valid_wallet_entropy, size, password, strlen(password), &wallet);
        TEST_ASSERT_EQUAL(0, wallet_rc);
        free(valid_wallet_entropy);
    }
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_can_create_address);
    RUN_TEST(invalid_entropy_size_returns_failure);
    RUN_TEST(valid_entropy_size_returns_success);
    return UNITY_END();
}
