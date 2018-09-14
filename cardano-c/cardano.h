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
cardano_transaction *cardano_transaction_builder_finalize(cardano_transaction_builder *tb);

cardano_transaction_finalized * cardano_transaction_finalized_new(cardano_transaction *c_tx);
cardano_result cardano_transaction_finalized_add_witness(cardano_transaction_finalized *tf, uint8_t c_xprv[96], uint32_t protocol_magic, uint8_t c_txid[32]);
cardano_signed_transaction *cardano_transaction_finalized_output(cardano_transaction_finalized *tf);

#ifdef __cplusplus
}
#endif

#endif
