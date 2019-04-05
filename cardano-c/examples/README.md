# Creating a wallet

This example shows how to create a wallet from english mnemonics

```C
char *MNEMONICS = "crowd captain hungry tray powder motor coast oppose month shed parent mystery torch resemble index";

/*Retrieve entropy from mnemonics*/
cardano_entropy entropy;
uint32_t bytes;
cardano_bip39_error_t entropy_rc = cardano_entropy_from_english_mnemonics(MNEMONICS, &entropy, &bytes);

/*Check that the mnemonics were actually valid*/
assert(entropy_rc == BIP39_SUCCESS);

/*Create a wallet with the given entropy*/
char *password = "password";
cardano_wallet *wallet;
cardano_result wallet_rc = cardano_wallet_new(entropy, bytes, password, strlen(password), &wallet);

assert(wallet_rc == CARDANO_RESULT_SUCCESS);

/*Create an account*/
const char *alias = "Awesome Account";
unsigned int index = 0;
cardano_account *account = cardano_account_create(wallet, alias, index);

/*Create an internal address*/
enum
{
    NUMBER_OF_ADDRESSES = 1,
};
char *address[NUMBER_OF_ADDRESSES];
const int IS_INTERNAL = 1;
const unsigned int FROM_INDEX = 0;
cardano_account_generate_addresses(account, IS_INTERNAL, FROM_INDEX, NUMBER_OF_ADDRESSES, address);

/*
    ...
*/

/*Release memory*/
cardano_delete_entropy_array(entropy, bytes);

cardano_account_delete_addresses(address, NUMBER_OF_ADDRESSES);

cardano_account_delete(account);

cardano_wallet_delete(wallet);
```