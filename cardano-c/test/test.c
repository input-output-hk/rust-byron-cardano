#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "cardano.h"
#include "unity/unity.h"

static const uint8_t static_wallet_entropy[16] = { 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15 };

void test_can_create_address(void) {
	static const char* alias = "Test Wallet";
	static char *address;

	cardano_wallet *wallet = cardano_wallet_new(static_wallet_entropy, 16, "abc", 3);

    TEST_ASSERT_MESSAGE(wallet, "The wallet creation failed");

    cardano_account *account = cardano_account_create(wallet, alias, 0);

    TEST_ASSERT_MESSAGE(account, "The account creation failed");

	cardano_account_generate_addresses(account, 0, 0, 1, &address);

    TEST_ASSERT_MESSAGE(!cardano_address_is_valid(address) , "The generated address is invalid");

	cardano_account_delete(account);

	cardano_wallet_delete(wallet);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_can_create_address);
    return UNITY_END();
}
