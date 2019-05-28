#include "../cardano.h"
#include <assert.h>
#include <string.h>
#include <stdio.h>

/*
Example of transaction generation an signing from the testnet
Two addresses are generated from the given mnemonics, in this case:

    - 2cWKMJemoBaiCnmPjDEhmPwDR9HhxtsLmzr4eHQNiQfBsRQ8sJnR94xGF5t8De5iBApqY
    - 2cWKMJemoBajYE5NxbHPzhuYy9KCXxqbkeFPdN1h5koATAhm2HqruhNwMdBKKmgCscBWw

The first one is used as the source address and is assumed that it has funds
of '1194911488' lovelaces in the output 0 of the transaction with id:

    "0090614e19a5fb74c41e4ac57e25ec0d41d44a55884eba14882ea8a403e59c24"

The output is written to a file with the resulting transaction id as the name
*/

//Protocol magic for testnet
static const uint32_t PROTOCOL_MAGIC = 1097911063;
const uint32_t BIP44_SOFT_UPPER_BOUND = 0x80000000;

int main(int argc, char *argv)
{
    char *MNEMONICS = "crowd captain hungry tray powder motor coast oppose month shed parent mystery torch resemble index";

    /*Retrieve entropy from mnemonics*/
    cardano_entropy entropy;
    uint32_t bytes;
    cardano_bip39_error_t entropy_rc = cardano_entropy_from_english_mnemonics(MNEMONICS, &entropy, &bytes);

    /*Check that the mnemonics were actually valid*/
    assert(entropy_rc == BIP39_SUCCESS);

    /*Create a wallet with the given entropy*/
    char *password = "";
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
        NUMBER_OF_ADDRESSES = 2,
    };
    char *addresses[NUMBER_OF_ADDRESSES];
    const int IS_INTERNAL = 0;
    const unsigned int FROM_INDEX = 0;
    cardano_account_generate_addresses(
        account,
        IS_INTERNAL,
        FROM_INDEX,
        NUMBER_OF_ADDRESSES,
        addresses,
        PROTOCOL_MAGIC
    );

    printf("%s\n%s\n", addresses[0], addresses[1]);

    /*Get the root key*/
    cardano_xprv *root_key = cardano_wallet_root_key(wallet);

    cardano_account_delete(account);
    cardano_wallet_delete(wallet);
    cardano_delete_entropy_array(entropy, bytes);

    /*Get a transaction builder*/
    cardano_transaction_builder *txbuilder = cardano_transaction_builder_new();

    /*
    Derive the private key to sign the transaction according to the 
    bip44 derivation path
    */

    //Derive the xprv of the account

    cardano_xprv *account_xprv = cardano_xprv_derive(
        root_key,
        BIP44_SOFT_UPPER_BOUND | 0
    );

    //BIP44 derivation path. 
    //0: External address
    cardano_xprv *external_address_level = cardano_xprv_derive(account_xprv, 0);

    //Derive the xprv of the first address
    cardano_xprv *input_xprv = cardano_xprv_derive(external_address_level, 0);

    //Byte representation is required for signing
    uint8_t *input_xprv_bytes = cardano_xprv_to_bytes(input_xprv);

    //Free the memory of the xprvs as only the input_xprv_bytes is needed later
    cardano_xprv_delete(input_xprv);
    cardano_xprv_delete(external_address_level);
    cardano_xprv_delete(account_xprv);
    cardano_xprv_delete(root_key);

    //Add the input

    /*The transaction with the unspent*/
    char *hex_unspent_txid = "0090614e19a5fb74c41e4ac57e25ec0d41d44a55884eba14882ea8a403e59c24";

    /*Parse the hex representation of txid*/
    uint8_t unspent_txid[32];

    for (size_t i = 0; i < sizeof unspent_txid; i++) {
        sscanf(hex_unspent_txid + (i*2), "%2hhx", unspent_txid + i);
    }

    cardano_txoptr *input = cardano_transaction_output_ptr_new(unspent_txid, 0);

    uint64_t input_funds = 1194911488;
    if (cardano_transaction_builder_add_input(txbuilder, input, input_funds)
     != CARDANO_RESULT_SUCCESS)
    {
        printf("Error adding input\n");
        return 1;
    }
    cardano_transaction_output_ptr_delete(input);

    //Transfer to the second generated address
    cardano_address *to_address = cardano_address_import_base58(addresses[1]);
    cardano_txoutput *output = cardano_transaction_output_new(to_address, 80000);
    cardano_address_delete(to_address);

    cardano_transaction_builder_add_output(txbuilder, output);
    cardano_transaction_output_delete(output);

    //Add the source address as change address
    cardano_address *change_addr = cardano_address_import_base58(addresses[0]);
    if (cardano_transaction_builder_add_change_addr(txbuilder, change_addr)
    != CARDANO_RESULT_SUCCESS) {
        printf("Error adding change address\n");
        return 1;
    }

    cardano_address_delete(change_addr);

    //Release the memory of the two base58 addresses
    cardano_account_delete_addresses(addresses, 2);

    cardano_transaction *tx;
    if (cardano_transaction_builder_finalize(txbuilder, &tx) != CARDANO_RESULT_SUCCESS)
    {
        printf("Error when finalizing transaction\n");
        return 1;
    }
    cardano_transaction_builder_delete(txbuilder);

    cardano_txid_t txid;
    cardano_transaction_txid(tx, &txid);

    cardano_transaction_finalized *tf = cardano_transaction_finalized_new(tx);

    if (cardano_transaction_finalized_add_witness(tf, input_xprv_bytes, PROTOCOL_MAGIC, txid.bytes)
        != CARDANO_RESULT_SUCCESS)
    {
        printf("Couldn't add witness\n");
        return 1;
    }

    cardano_xprv_bytes_delete(input_xprv_bytes);
    cardano_transaction_delete(tx);

    cardano_signed_transaction *txaux;
    if (cardano_transaction_finalized_output(tf, &txaux) != CARDANO_RESULT_SUCCESS)
    {
        printf("Error in finalized output\n");
        return 1;
    }
    cardano_transaction_finalized_delete(tf);

    uint8_t *serialized_bytes;
    size_t serialized_size;
    if(cardano_signed_transaction_serialize(txaux, &serialized_bytes, &serialized_size) 
    != CARDANO_RESULT_SUCCESS) {
        printf("Error when serializing the transaction\n");
        return 1;
    }
    cardano_transaction_signed_delete(txaux);

    //Print the resulting txid in hexadecimal notation
    char txid_str[sizeof(txid) * 2 + 1];
    for (unsigned int j = 0; j < sizeof(txid); ++j)
    {
        sprintf(txid_str + (j*2), "%02x", txid.bytes[j]);
    }

    //Write the binary encoded transaction to file
    FILE *file = fopen(txid_str, "w");
    fwrite(serialized_bytes, sizeof(uint8_t), serialized_size, file);
    fclose(file);

    cardano_signed_transaction_serialized_delete(serialized_bytes, serialized_size);
}