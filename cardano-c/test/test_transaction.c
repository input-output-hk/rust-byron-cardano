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
cardano_txoptr *input;
cardano_txoutput *output;

//Constants
static uint32_t PROTOCOL_MAGIC = 1;
static uint8_t input_xprv[XPRV_SIZE] = {0};
static const uint8_t static_wallet_entropy[16] = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15};
static uint8_t txid[32] = {0};
const uint64_t MAX_COIN = 45000000000000000;

void setUp()
{
    cardano_result wallet_rc = cardano_wallet_new(
        static_wallet_entropy,
        sizeof(static_wallet_entropy),
        "password",
        strlen("password"),
        &wallet);

    account = cardano_account_create(wallet, "main", 0);

    char *addresses[2];
    size_t NUMBER_OF_ADDRESSES = sizeof(addresses) / sizeof(char *);

    int rc = cardano_account_generate_addresses(account, 0, 0, NUMBER_OF_ADDRESSES, addresses);

    input_address = cardano_address_import_base58(addresses[0]);
    output_address = cardano_address_import_base58(addresses[1]);

    cardano_account_delete_addresses(addresses, sizeof(addresses) / sizeof(char *));

    txbuilder = cardano_transaction_builder_new();
    
    input = cardano_transaction_output_ptr_new(txid, 1);
    output = cardano_transaction_output_new(output_address, 1000);
}

void tearDown()
{
    cardano_transaction_output_delete(output);

    cardano_transaction_output_ptr_delete(input);

    cardano_wallet_delete(wallet);

    cardano_transaction_builder_delete(txbuilder);

    cardano_address_delete(input_address);

    cardano_address_delete(output_address);

    cardano_account_delete(account);
}

void test_add_input_returns_success_with_valid_value()
{
    cardano_transaction_error_t irc = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    TEST_ASSERT_EQUAL(CARDANO_RESULT_SUCCESS, irc);
}

void test_add_input_returns_error_with_big_value()
{
    cardano_transaction_error_t irc = cardano_transaction_builder_add_input(txbuilder, input, MAX_COIN + 1);

    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_COIN_OUT_OF_BOUNDS, irc);
}

void test_add_witness_returns_error_with_less_inputs()
{
    cardano_result irc = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    /* the builder finalize fails without outputs*/
    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction *tx; cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);

    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_SUCCESS, tx_rc);

    cardano_transaction_finalized *tf = cardano_transaction_finalized_new(tx);

    cardano_transaction_error_t rc1 = cardano_transaction_finalized_add_witness(tf, input_xprv, PROTOCOL_MAGIC, txid);

    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_SUCCESS, rc1);

    cardano_transaction_error_t rc2 = cardano_transaction_finalized_add_witness(tf, input_xprv, PROTOCOL_MAGIC, txid);

    //#witnesses > #inputs
    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_SIGNATURES_EXCEEDED, rc2);

    cardano_transaction_delete(tx);
    cardano_transaction_finalized_delete(tf);
}

void test_builder_finalize_error_code_no_inputs()
{
    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);
    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_NO_INPUT, tx_rc);
}

void test_builder_finalize_error_code_no_outputs()
{
    cardano_transaction_error_t irc = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);
    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_NO_OUTPUT, tx_rc);
}

void test_transaction_finalized_output_error_code_signature_mismatch()
{
    cardano_transaction_error_t irc1 = cardano_transaction_builder_add_input(txbuilder, input, 1000);
    cardano_transaction_error_t irc2 = cardano_transaction_builder_add_input(txbuilder, input, 1000);

    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);

    cardano_transaction_finalized *tf = cardano_transaction_finalized_new(tx);

    cardano_transaction_error_t rc1 = cardano_transaction_finalized_add_witness(tf, input_xprv, PROTOCOL_MAGIC, txid);

    cardano_signed_transaction *txaux;
    cardano_transaction_error_t rc = cardano_transaction_finalized_output(tf, &txaux);

    //#inputs (2) > #witnesses (1)
    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_SIGNATURE_MISMATCH, rc);

    cardano_transaction_delete(tx);
    cardano_transaction_finalized_delete(tf);
}

void test_transaction_finalized_output_success()
{
    cardano_transaction_error_t irc1 = cardano_transaction_builder_add_input(txbuilder, input, 1000);
    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction *tx;
    cardano_transaction_error_t tx_rc = cardano_transaction_builder_finalize(txbuilder, &tx);

    cardano_transaction_finalized *tf = cardano_transaction_finalized_new(tx);

    cardano_transaction_error_t rc1 = cardano_transaction_finalized_add_witness(tf, input_xprv, PROTOCOL_MAGIC, txid);

    cardano_signed_transaction *txaux;
    cardano_transaction_error_t rc = cardano_transaction_finalized_output(tf, &txaux);

    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_SUCCESS, rc);

    cardano_transaction_delete(tx);
    cardano_transaction_finalized_delete(tf);
    cardano_transaction_signed_delete(txaux);
}

void test_transaction_balance_positive() {
    cardano_transaction_coin_diff_t *balance;

    cardano_transaction_builder_add_input(txbuilder, input, 1000000);

    cardano_transaction_error_t rc = cardano_transaction_builder_balance(txbuilder, &balance);
    uint64_t fee = cardano_transaction_builder_fee(txbuilder);

    TEST_ASSERT_EQUAL(1000000 - fee, (*balance).value);
    //TEST_ASSERT_EQUAL(DIFF_POSITIVE, (*balance).sign);
}

void test_transaction_balance_negative() {
    cardano_transaction_coin_diff_t *balance;
    cardano_transaction_error_t rc = cardano_transaction_builder_balance(txbuilder, &balance);

    uint64_t fee = cardano_transaction_builder_fee(txbuilder);

    TEST_ASSERT_EQUAL(fee, (*balance).value);
    TEST_ASSERT_EQUAL(DIFF_NEGATIVE, (*balance).sign);
}

void test_transaction_balance_zero() {
    enum {
        BIG_VALUE_TO_COVER_FEE = 10000000,
    };
    cardano_transaction_builder_add_input(txbuilder, input, BIG_VALUE_TO_COVER_FEE);
    cardano_result add_change_rc = cardano_transaction_builder_add_change_addr(txbuilder, output_address);

    cardano_transaction_coin_diff_t *balance;
    cardano_transaction_error_t rc = cardano_transaction_builder_balance(txbuilder, &balance);

    TEST_ASSERT_EQUAL(0, (*balance).value);
    TEST_ASSERT_EQUAL(DIFF_ZERO, (*balance).sign);
}

void test_transaction_builder_balance_too_big() {
    cardano_txoptr *input1 = cardano_transaction_output_ptr_new(txid, 1);
    cardano_txoptr *input2 = cardano_transaction_output_ptr_new(txid, 2);

    cardano_result irc1 = cardano_transaction_builder_add_input(txbuilder, input1, MAX_COIN);
    cardano_result irc2 = cardano_transaction_builder_add_input(txbuilder, input1, 1);

    cardano_transaction_coin_diff_t *balance; 
    cardano_transaction_error_t brc1 = cardano_transaction_builder_balance(txbuilder, &balance);

    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_COIN_OUT_OF_BOUNDS, brc1);

    cardano_transaction_output_ptr_delete(input1);
    cardano_transaction_output_ptr_delete(input2);
}

void test_transaction_builder_balance_without_fee_too_big() {
    cardano_txoptr *input1 = cardano_transaction_output_ptr_new(txid, 1);
    cardano_txoptr *input2 = cardano_transaction_output_ptr_new(txid, 2);

    cardano_result irc1 = cardano_transaction_builder_add_input(txbuilder, input1, MAX_COIN);
    cardano_result irc2 = cardano_transaction_builder_add_input(txbuilder, input1, 1);

    cardano_transaction_coin_diff_t *balance; 
    cardano_transaction_error_t brc1 = cardano_transaction_builder_balance_without_fees(txbuilder, &balance);

    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_COIN_OUT_OF_BOUNDS, brc1);

    cardano_transaction_output_ptr_delete(input1);
    cardano_transaction_output_ptr_delete(input2);
}

void test_transaction_balance_without_fee_positive() {
    cardano_transaction_builder_add_input(txbuilder, input, 1000);
    cardano_transaction_coin_diff_t *balance;
    cardano_transaction_error_t rc = cardano_transaction_builder_balance_without_fees(txbuilder, &balance);

    TEST_ASSERT_EQUAL(1000, (*balance).value);
    TEST_ASSERT_EQUAL(DIFF_POSITIVE, (*balance).sign);
}

void test_transaction_balance_without_fee_negative() {
    cardano_txoutput *output = cardano_transaction_output_new(output_address, 1000);

    cardano_transaction_builder_add_output(txbuilder, output);
    cardano_transaction_coin_diff_t *balance;
    cardano_transaction_error_t rc = cardano_transaction_builder_balance_without_fees(txbuilder, &balance);

    TEST_ASSERT_EQUAL(1000, (*balance).value);
    TEST_ASSERT_EQUAL(DIFF_NEGATIVE, (*balance).sign);
    cardano_transaction_output_delete(output);
}

void test_transaction_balance_without_fee_zero() {
    cardano_txoutput *output = cardano_transaction_output_new(output_address, 1000);

    cardano_transaction_builder_add_input(txbuilder, input, 1000);
    cardano_transaction_builder_add_output(txbuilder, output);

    cardano_transaction_coin_diff_t *balance;
    cardano_transaction_error_t rc = cardano_transaction_builder_balance_without_fees(txbuilder, &balance);

    TEST_ASSERT_EQUAL(0, (*balance).value);
    TEST_ASSERT_EQUAL(DIFF_ZERO, (*balance).sign);
    cardano_transaction_output_delete(output);
}

void test_transaction_get_input_total() {
    cardano_transaction_error_t irc = cardano_transaction_builder_add_input(txbuilder, input, 1000);
    uint64_t input_total;
    cardano_transaction_error_t rc = cardano_transaction_builder_get_input_total(txbuilder, &input_total);
    TEST_ASSERT_EQUAL(1000, input_total);
}

void test_transaction_get_output_total() {
    cardano_transaction_builder_add_output(txbuilder, output);
    uint64_t output_total;
    cardano_transaction_error_t rc = cardano_transaction_builder_get_output_total(txbuilder, &output_total);
    TEST_ASSERT_EQUAL(1000, output_total);
}

void test_transaction_get_input_total_no_inputs() {
    uint64_t input_total;
    cardano_transaction_error_t rc = cardano_transaction_builder_get_input_total(txbuilder, &input_total);
    TEST_ASSERT_EQUAL(0, input_total);
}

void test_transaction_get_output_total_no_outputs() {
    uint64_t output_total;
    cardano_transaction_error_t rc = cardano_transaction_builder_get_output_total(txbuilder, &output_total);
    TEST_ASSERT_EQUAL(0, output_total);
}

void test_transaction_get_input_total_too_big()
{
    cardano_transaction_error_t irc1 = cardano_transaction_builder_add_input(txbuilder, input, MAX_COIN);
    cardano_transaction_error_t irc2 = cardano_transaction_builder_add_input(txbuilder, input, 1);
    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_SUCCESS, irc1);
    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_SUCCESS, irc2);

    uint64_t input_total;
    cardano_transaction_error_t rc = cardano_transaction_builder_get_input_total(txbuilder, &input_total);
    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_COIN_OUT_OF_BOUNDS, rc);
}

void test_transaction_get_output_total_too_big()
{
    cardano_txoutput *output1 = cardano_transaction_output_new(output_address, MAX_COIN);
    cardano_txoutput *output2 = cardano_transaction_output_new(output_address, 1);

    cardano_transaction_builder_add_output(txbuilder, output1);
    cardano_transaction_builder_add_output(txbuilder, output2);
    uint64_t output_total;
    cardano_transaction_error_t rc = cardano_transaction_builder_get_output_total(txbuilder, &output_total);
    TEST_ASSERT_EQUAL(CARDANO_TRANSACTION_COIN_OUT_OF_BOUNDS, rc);

    cardano_transaction_output_delete(output1);
    cardano_transaction_output_delete(output2);
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
    RUN_TEST(test_transaction_balance_zero);
    RUN_TEST(test_transaction_balance_negative);
    RUN_TEST(test_transaction_balance_positive);
    RUN_TEST(test_transaction_builder_balance_too_big);
    RUN_TEST(test_transaction_balance_without_fee_zero);
    RUN_TEST(test_transaction_balance_without_fee_negative);
    RUN_TEST(test_transaction_balance_without_fee_positive);
    RUN_TEST(test_transaction_builder_balance_without_fee_too_big);
    RUN_TEST(test_transaction_get_input_total);
    RUN_TEST(test_transaction_get_input_total_no_inputs);
    RUN_TEST(test_transaction_get_output_total);
    RUN_TEST(test_transaction_get_output_total_no_outputs);
    RUN_TEST(test_transaction_get_input_total_too_big);
    RUN_TEST(test_transaction_get_output_total_too_big);
    return UNITY_END();
}