#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "cardano.h"

static const uint8_t static_wallet_entropy[16] = { 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15 };

int wallet_test(void) {
	static const char* alias = "Test Wallet";
	static char *address;

	cardano_wallet *wallet = cardano_wallet_new(static_wallet_entropy, 16, "abc", 3);
	if (!wallet) goto error;

	cardano_account *account = cardano_account_create(wallet, alias, 0);
	if (!account) goto error;

	cardano_account_generate_addresses(account, 0, 0, 1, &address);

	printf("address generated: %s\n", address);

	printf("address is valid: %s\n", cardano_address_is_valid(address) ? "NO" : "YES");

	cardano_account_delete(account);

	cardano_wallet_delete(wallet);

	return 0;
error:
	return -1;
}

int main(int argc, char* argv[]) {
	if (wallet_test()) exit(35);
	return 0;
}
