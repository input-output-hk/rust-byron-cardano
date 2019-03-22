#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include "../cardano.h"
#include "unity/unity.h"

//Variables for the setUp function
cardano_wallet *wallet;
cardano_account *account;
cardano_address *input_address;
cardano_address *output_address;
cardano_transaction_builder *txbuilder;

//Constants
static uint32_t PROTOCOL_MAGIC = 1;
static uint8_t input_xprv[XPRV_SIZE] = {0};
static const uint8_t static_wallet_entropy[16] = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15};
static uint8_t txid[32] = {0};

void setUp()
{
    wallet = cardano_wallet_new(
        static_wallet_entropy,
        sizeof(static_wallet_entropy),
        "password",
        strlen("password"));

    account = cardano_account_create(wallet, "main", 0);

    char *addresses[2];
    size_t NUMBER_OF_ADDRESSES = sizeof(addresses) / sizeof(char *);

    int rc = cardano_account_generate_addresses(account, 0, 0, NUMBER_OF_ADDRESSES, addresses);

    input_address = cardano_address_import_base58(addresses[0]);
    output_address = cardano_address_import_base58(addresses[1]);

    cardano_account_delete_addresses(addresses, sizeof(addresses) / sizeof(char *));

    txbuilder = cardano_transaction_builder_new();
}

void tearDown()
{
    cardano_account_delete(account);

    cardano_wallet_delete(wallet);

    cardano_address_delete(input_address);

    cardano_address_delete(output_address);

    cardano_transaction_builder_delete(txbuilder);
}

void test_add_input_returns_success_with_valid_value()
{
    cardano_txoptr *input = cardano_transaction_output_ptr_new(txid, 1);
    cardano_transaction_error_t irc = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    TEST_ASSERT_EQUAL(CARDANO_RESULT_SUCCESS, irc);
    cardano_transaction_output_ptr_delete(input);
}

void test_add_input_returns_error_with_big_value()
{
    const uint64_t MAX_COIN = 45000000000000000;
    cardano_txoptr *input = cardano_transaction_output_ptr_new(txid, 1);
    cardano_transaction_error_t irc = cardano_transaction_builder_add_input(txbuilder, input, MAX_COIN + 1);

    TEST_ASSERT_EQUAL(TRANSACTION_COIN_OUT_OF_BOUNDS, irc);
    cardano_transaction_output_ptr_delete(input);
}

void test_add_witness_returns_error_with_less_inputs()
{
    cardano_txoptr *input = cardano_transaction_output_ptr_new(txid, 1);
    cardano_result irc = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    /* the builder finalize fails without outputs*/
    cardano_txoutput *output = cardano_transaction_output_new(output_address, 1000);
    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);

    TEST_ASSERT_EQUAL(TRANSACTION_SUCCESS, tx_rc);

    cardano_transaction_finalized *tf = cardano_transaction_finalized_new(tx);

    cardano_transaction_error_t rc1 = cardano_transaction_finalized_add_witness(tf, input_xprv, PROTOCOL_MAGIC, txid);

    TEST_ASSERT_EQUAL(TRANSACTION_SUCCESS, rc1);

    cardano_transaction_error_t rc2 = cardano_transaction_finalized_add_witness(tf, input_xprv, PROTOCOL_MAGIC, txid);

    //#witnesses > #inputs
    TEST_ASSERT_EQUAL(TRANSACTION_SIGNATURES_EXCEEDED, rc2);

    cardano_transaction_output_ptr_delete(input);
    cardano_transaction_output_delete(output);
    cardano_transaction_delete(tx);
    cardano_transaction_finalized_delete(tf);
}

void test_builder_finalize_error_code_no_inputs()
{
    cardano_txoutput *output = cardano_transaction_output_new(output_address, 1000);
    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);
    TEST_ASSERT_EQUAL(TRANSACTION_NO_INPUT, tx_rc);

    cardano_transaction_output_delete(output);
}

void test_builder_finalize_error_code_no_outputs()
{
    cardano_txoptr *input = cardano_transaction_output_ptr_new(txid, 1);
    cardano_transaction_error_t irc = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);
    TEST_ASSERT_EQUAL(TRANSACTION_NO_OUTPUT, tx_rc);

    cardano_transaction_output_ptr_delete(input);
}

void test_transaction_finalized_output_error_code_signature_mismatch()
{
    cardano_txoptr *input = cardano_transaction_output_ptr_new(txid, 1);

    cardano_transaction_error_t irc1 = cardano_transaction_builder_add_input(txbuilder, input, 1000);
    cardano_transaction_error_t irc2 = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    cardano_txoutput *output = cardano_transaction_output_new(output_address, 1000);
    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);

    cardano_transaction_finalized *tf = cardano_transaction_finalized_new(tx);

    cardano_transaction_error_t rc1 = cardano_transaction_finalized_add_witness(tf, input_xprv, PROTOCOL_MAGIC, txid);

    cardano_signed_transaction *txaux;
    cardano_transaction_error_t rc = cardano_transaction_finalized_output(tf, &txaux);

    //#inputs (2) > #witnesses (1)
    TEST_ASSERT_EQUAL(TRANSACTION_SIGNATURE_MISMATCH, rc);

    cardano_transaction_output_ptr_delete(input);
    cardano_transaction_output_delete(output);
    cardano_transaction_delete(tx);
    cardano_transaction_finalized_delete(tf);
}

void test_transaction_finalized_output_success()
{
    cardano_txoptr *input = cardano_transaction_output_ptr_new(txid, 1);

    cardano_transaction_error_t irc1 = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    cardano_txoutput *output = cardano_transaction_output_new(output_address, 1000);
    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);

    cardano_transaction_finalized *tf = cardano_transaction_finalized_new(tx);

    cardano_transaction_error_t rc1 = cardano_transaction_finalized_add_witness(tf, input_xprv, PROTOCOL_MAGIC, txid);

    cardano_signed_transaction *txaux;
    cardano_transaction_error_t rc = cardano_transaction_finalized_output(tf, &txaux);

    cardano_transaction_output_ptr_delete(input);
    cardano_transaction_output_delete(output);
    cardano_transaction_delete(tx);
    cardano_transaction_finalized_delete(tf);
    cardano_transaction_signed_delete(txaux);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_add_input_returns_success_with_valid_value);
    RUN_TEST(test_add_input_returns_error_with_big_value);
    RUN_TEST(test_add_witness_returns_error_with_less_inputs);
    RUN_TEST(test_builder_finalize_error_code_no_inputs);
    RUN_TEST(test_builder_finalize_error_code_no_outputs);
    RUN_TEST(test_transaction_finalized_output_error_code_signature_mismatch);
    RUN_TEST(test_transaction_finalized_output_success);
    return UNITY_END();
}