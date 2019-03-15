#ifndef CARDANO_RUST_H
# define CARDANO_RUST_H
/* Basic Types */

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

typedef int cardano_result;

/*********/
/* BIP39 */
/*********/

/* bip39 error definitions */
typedef enum _bip39_config_error
{
    BIP39_SUCCESS = 0,
    BIP39_INVALID_MNEMONIC = 1,
    BIP39_INVALID_CHECKSUM = 2,
    BIP39_INVALID_WORD_COUNT = 3
} cardano_bip39_error_t;

typedef uint8_t* cardano_entropy;

/*!
* \brief get entropy array from the given english mnemonics 
* \param [in] mnemonics a string consisting of 9, 12, 15, 18, 21 or 24 english words
* \param [out] entropy the returned entropy array
* \param [out] entropy_size the size of the the returned array
* \returns BIP39_SUCCESS or either BIP39_INVALID_MNEMONIC or BIP39_INVALID_CHECKSUM 
*/
cardano_bip39_error_t cardano_entropy_from_english_mnemonics(
    const char *mnemonics,
    cardano_entropy *entropy,
    uint32_t *entropy_size
);

/*!
* \brief encode a entropy into its equivalent words represented by their index (0 to 2047) in the BIP39 dictionary
* \param [in] number_of_words one of 9, 12, 15, 18, 21 or 24 representing the number of words of the equivalent mnemonic
* \param [in] random_generator a function that generates random bytes  
* \param [out] entropy the returned entropy array
* \param [out] entropy_size the size of the the returned array
* \returns BIP39_SUCCESS or BIP39_INVALID_WORD_COUNT 
*/
cardano_bip39_error_t cardano_entropy_from_random(
    uint8_t number_of_words,
    uint8_t (*random_generator)(),
    cardano_entropy *entropy,
    uint32_t *entropy_size
);

/*!
* delete the allocated memory of entropy byte array
* \param [in] entropy the entropy array
* \param [in] entropy_size the length of the entropy array
* \sa cardano_entropy_from_random()
* \sa cardano_entropy_from_english_mnemonics()
*/
void cardano_delete_entropy_array(uint8_t *entropy, uint32_t entropy_size);

cardano_result cardano_bip39_encode(const char * const entropy_raw, unsigned long entropy_size, unsigned short *mnemonic_index, unsigned long mnemonic_size);

/*********/
/* Keys  */
/*********/

#define XPRV_SIZE 96

typedef struct cardano_xprv cardano_xprv;
typedef struct cardano_xpub cardano_xpub;

cardano_xpub *cardano_xprv_delete(cardano_xprv *privkey);
cardano_xpub *cardano_xprv_to_xpub(cardano_xprv *privkey);

uint8_t *cardano_xprv_to_bytes(cardano_xprv *privkey);
cardano_xprv *cardano_xprv_from_bytes(uint8_t bytes[XPRV_SIZE]);

cardano_xpub *cardano_xpub_delete(cardano_xpub *pubkey);

/*************/
/* addresses */
/*************/

typedef struct cardano_address cardano_address;

/* check if an address is a valid protocol address.
 * return 0 on success, !0 on failure. */
int cardano_address_is_valid(const char * address_base58);

cardano_address *cardano_address_new_from_pubkey(cardano_xpub *publickey);
void cardano_address_delete(cardano_address *address);

char *cardano_address_export_base58(cardano_address *address);
cardano_address *cardano_address_import_base58(const char * address_bytes);

/***********/
/* Wallet  */
/***********/

typedef struct cardano_wallet cardano_wallet;
typedef struct cardano_account cardano_account;

cardano_wallet *cardano_wallet_new(const uint8_t * const entropy_ptr, unsigned long entropy_size,
                                   const char * const password_ptr, unsigned long password_size);
void cardano_wallet_delete(cardano_wallet *);

cardano_account *cardano_account_create(cardano_wallet *wallet, const char *alias, unsigned int index);
void cardano_account_delete(cardano_account *account);

unsigned long cardano_account_generate_addresses(cardano_account *account, int internal, unsigned int from_index, unsigned long num_indices, char *addresses_ptr[]);

/****************/
/* Transactions */
/****************/

typedef struct cardano_transaction_builder cardano_transaction_builder;
typedef struct cardano_transaction_finalized cardano_transaction_finalized;
typedef struct cardano_txoptr cardano_txoptr;
typedef struct cardano_txoutput cardano_txoutput;
typedef struct cardano_txoutput cardano_txoutput;
typedef struct cardano_transaction cardano_transaction;
typedef struct cardano_signed_transaction cardano_signed_transaction;

cardano_txoptr * cardano_transaction_output_ptr_new(uint8_t txid[32], uint32_t index);
void cardano_transaction_output_ptr_delete(cardano_txoptr *txo);

cardano_txoutput * cardano_transaction_output_new(cardano_address *c_addr, uint64_t value);
void cardano_transaction_output_delete(cardano_txoutput *output);

cardano_transaction_builder * cardano_transaction_builder_new(void);
void cardano_transaction_builder_delete(cardano_transaction_builder *tb);
void cardano_transaction_builder_add_output(cardano_transaction_builder *tb, cardano_txoptr *txo);
cardano_result cardano_transaction_builder_add_input(cardano_transaction_builder *tb, cardano_txoptr *c_txo, uint64_t value);
cardano_result cardano_transaction_builder_add_change_addr(cardano_transaction_builder *tb, cardano_address *change_addr);
uint64_t cardano_transaction_builder_fee(cardano_transaction_builder *tb);
cardano_transaction *cardano_transaction_builder_finalize(cardano_transaction_builder *tb);

cardano_transaction_finalized * cardano_transaction_finalized_new(cardano_transaction *c_tx);
cardano_result cardano_transaction_finalized_add_witness(cardano_transaction_finalized *tf, uint8_t c_xprv[96], uint32_t protocol_magic, uint8_t c_txid[32]);
cardano_signed_transaction *cardano_transaction_finalized_output(cardano_transaction_finalized *tf);

#ifdef __cplusplus
}
#endif

#endif
