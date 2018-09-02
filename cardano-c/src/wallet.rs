use cardano::wallet::scheme::{Wallet};
use cardano::wallet::bip44;
use cardano::hdwallet;
use cardano::bip;
use cardano::address;
use cardano::util;

use std::os::raw::{c_char};
use std::{ffi, slice, ptr};

/* ******************************************************************************* *
 *                                  Wallet object                                  *
 * ******************************************************************************* */

/// handy type alias for pointer to a heap allocated wallet
type WalletPtr  = *mut bip44::Wallet;
/// handy type alias for pointer to a heap allocated account
type AccountPtr = *mut bip44::Account<hdwallet::XPub>;

// TODO: one of the major missing element is a proper clean error handling

/// Create a HD BIP44 compliant Wallet from the given entropy and a password
///
/// Password can be empty
///
/// use the function `cardano_wallet_delete` to free all the memory associated to the returned
/// object. This function may fail if:
///
/// - panic: if there is no more memory to allocate the object to return
/// - panic or return 0 (nullptr or NULL) if the given seed_ptr is of invalid length
///
#[no_mangle]
pub extern "C"
fn cardano_wallet_new( entropy_ptr: *const u8 /* expecting entropy ptr ... */
                     , entropy_size: usize    /* entropy size */
                     , password_ptr: *const u8 /* password ptr */
                     , password_size: usize    /* password size */
                     ) -> WalletPtr
{
    let entropy_slice = unsafe { slice::from_raw_parts(entropy_ptr, entropy_size) };
    let password = unsafe { slice::from_raw_parts(password_ptr, password_size) };

    let entropy = match bip::bip39::Entropy::from_slice(entropy_slice) {
        Err(_) => return ptr::null_mut(),
        Ok(e) => e,
    };

    let wallet = bip44::Wallet::from_entropy(&entropy, &password, hdwallet::DerivationScheme::V2);

    let wallet_box = Box::new(wallet);
    Box::into_raw(wallet_box)
}

/// take ownership of the given pointer and free the associated data
///
/// The data must be a valid Wallet created by `cardano_wallet_new`.
#[no_mangle]
pub extern "C"
fn cardano_wallet_delete(wallet_ptr: WalletPtr)
{
    unsafe {
        Box::from_raw(wallet_ptr)
    };
}

/* ******************************************************************************* *
 *                                 Account object                                  *
 * ******************************************************************************* */

/// create a new account, the account is given an alias and an index,
/// the index is the derivation index, we do not check if there is already
/// an account with this given index. The alias here is only an handy tool
/// to retrieve a created account from a wallet.
///
/// The returned object is not owned by any smart pointer or garbage collector.
/// To avoid memory leak, use `cardano_account_delete`
///
#[no_mangle]
pub extern "C"
fn cardano_account_create( wallet_ptr: WalletPtr
                         , account_alias: *mut c_char
                         , account_index: u32
                         )
    -> AccountPtr
{
    let wallet = unsafe { wallet_ptr.as_mut() }.expect("Not a NULL PTR");
    let account_alias = unsafe {
        ffi::CStr::from_ptr(account_alias).to_string_lossy()
    };

    let account = wallet.create_account(&account_alias, account_index);
    let account = Box::new(account.public());

    Box::into_raw(account)
}

/// take ownership of the given pointer and free the memory associated
#[no_mangle]
pub extern "C"
fn cardano_account_delete(account_ptr: AccountPtr)
{
    unsafe {
        Box::from_raw(account_ptr)
    };
}

#[no_mangle]
pub extern "C"
fn cardano_account_generate_addresses( account_ptr:  AccountPtr
                                     , internal:     bool
                                     , from_index: u32
                                     , num_indices: usize
                                     , addresses_ptr: *mut *mut c_char
                                     )
    -> usize
{
    let account = unsafe { account_ptr.as_mut() }
        .expect("Not a NULL PTR");

    let addr_type = if internal {
        bip44::AddrType::Internal
    } else {
        bip44::AddrType::External
    };

    account.address_generator(addr_type, from_index)
        .expect("we expect the derivation to happen successfully")
        .take(num_indices)
        .enumerate()
        .map(|(idx, xpub)| {
            let address = address::ExtendedAddr::new_simple(*xpub.unwrap());
            let address = format!("{}", util::base58::encode(&address.to_bytes()));
            // generate a C String (null byte terminated string)
            let c_address = ffi::CString::new(address)
                .expect("base58 strings only contains ASCII chars");
            // make sure the ptr is stored at the right place with alignments and all
            unsafe {
                ptr::write(addresses_ptr.wrapping_offset(idx as isize), c_address.into_raw())
            };
        }).count()
}
