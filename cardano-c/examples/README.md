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

# [Http bridge](https://github.com/input-output-hk/cardano-http-bridge) integration

To read the transactions on the blockchain, one can query the blocks from the bridge in its raw form and then use the C bindings to read this data. For example:
## Decoding a block

```C
/*main.c*/
/* ... */
#include "cardano.h"

/*
Assumming that raw_block is an array of bytes containing the data
obtained from the http bridge with the GET /:network/block/:blockid endpoint
and the raw_block_size contains the respective size of this buffer

char *raw_block;
size_t raw_block_size;
*/

int main(int argc, char *argv[]) {
    cardano_block *block;
    cardano_raw_block_decode(raw_block, raw_block_size, &block);
    //free(raw_block);

    print_block(block);
    return 0;
}
```

## Printing block information

```C
void print_block(cardano_block *block) {
    cardano_block_header *header = cardano_block_get_header(block);

    char *hash = cardano_block_header_compute_hash(header);
    printf("Block id: %s\n", hash);
    cardano_block_delete_hash(hash);

    char *previous_hash = cardano_block_header_previous_hash(header);
    printf("Previous block id: %s\n", previous_hash);
    cardano_block_delete_hash(previous_hash);

    cardano_block_header_delete(header);

    size_t transactions_size;
    cardano_signed_transaction **transactions;
    cardano_result rc = cardano_block_get_transactions(block, &transactions, &transactions_size);

    assert(rc == CARDANO_RESULT_SUCCESS);

    printf("Transactions: (%zu)\n", transactions_size);
    for (unsigned int i = 0; i < transactions_size; ++i)
    {
        print_transaction(transactions[i]);
    }

    cardano_block_delete_transactions(transactions, transactions_size);
    cardano_block_delete(block);
}
```

## Printing transactions

```C
void print_transaction(cardano_signed_transaction *tx) {
    print_inputs(tx);
    print_outputs(tx);
}
```

```C
void print_inputs(cardano_signed_transaction *tx)
{
    printf("Inputs\n");
    cardano_txoptr **inputs;
    size_t inputs_size;

    cardano_signed_transaction_get_inputs(tx, &inputs, &inputs_size);

    for (unsigned int i = 0; i < inputs_size; ++i)
    {
        uint32_t index = cardano_transaction_txoptr_index(inputs[i]);
        cardano_txid_t txid;
        cardano_transaction_txoptr_txid(inputs[i], &txid);

        /*Print the array of bytes as a hex string*/
        printf("Txid:");
        for (unsigned int j = 0; j < sizeof(txid); ++j)
        {
            printf("%02x", txid.bytes[j]);
        }
        printf("\n");

        /*The index in the tx*/
        printf("Offset %d\n", index);
    }
    cardano_signed_transaction_delete_inputs(inputs, inputs_size);
}
```

```C
void print_outputs(cardano_signed_transaction *tx)
{
    printf("Outputs\n");
    cardano_txoutput **outputs;
    size_t outputs_size;

    cardano_signed_transaction_get_outputs(tx, &outputs, &outputs_size);

    for (unsigned int i = 0; i < outputs_size; ++i)
    {
        cardano_address *address = cardano_transaction_txoutput_address(outputs[i]);
        char *address_base58 = cardano_address_export_base58(address);
        uint64_t value = cardano_transaction_txoutput_value(outputs[i]);
        printf("Value: %" PRIu64 "\n", value);
        printf("Address: %s\n", address_base58);
        cardano_account_delete_addresses(&address_base58, 1);
    }

    cardano_signed_transaction_delete_outputs(outputs, outputs_size);
}
```