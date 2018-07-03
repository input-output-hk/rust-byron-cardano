use cardano::{bip39, paperwallet, wallet};
use rand;

use std::io::{Write, stdout, stdin};
use dialoguer::{PasswordInput, Input, Confirmation};
use console::{Term};

use super::config;

#[cfg(unix)]
pub fn get_password() -> String {
    PasswordInput::new("password")
        .interact_on(&Term::stdout())
        .unwrap()
}
#[cfg(windows)]
pub fn get_password() -> String {
    Input::new("password")
        .interact_on(&Term::stdout())
        .unwrap()
}

#[cfg(unix)]
pub fn new_password() -> String {
    println!("Enter you wallet password. It will be needed to recover");
    println!("your wallet later with the mnemonic phrase.");

    PasswordInput::new("password")
        .confirm("Confirm password", "Passwords mismatching")
        .interact_on(&Term::stdout())
        .unwrap()
}
#[cfg(windows)]
pub fn new_password() -> String {
    println!("Enter you wallet password. It will be needed to recover");
    println!("your wallet later with the mnemonic phrase.");

    let pwd1 = Input::new("password")
        .interact()
        .unwrap();
    let pwd2 = Input::new("Confirm password")
        .interact()
        .unwrap();
    if pwd1 != pwd2 {
        panic!("Passwords mismatching");
    }
    pwd1
}
 // own receive napkin fame episode mimic hard crucial river vintage cool average source grow wash
#[cfg(unix)]
fn read_word(index: usize) -> String {
    let prompt = format!("mnemonic {}: ", index);
    let word = PasswordInput::new(&prompt).interact_on(&Term::stdout()).unwrap();
    Term::stdout().clear_line().unwrap();
    word
}
#[cfg(windows)]
fn read_word(index: usize) -> String {
    let prompt = format!("mnemonic {}: ", index);
    let word = Input::new(&prompt).interact_on(&Term::stdout()).unwrap();
    word
}

pub fn get_mnemonic_word<D>(index: usize, dic: &D) -> Option<bip39::Mnemonic>
    where D: bip39::dictionary::Language
{
    let mut mnemonic = None;

    for _ in 0..3 {
        let word = read_word(index);
        if word == "finished" || word.is_empty() {
            let done = Confirmation::new("No mnemonic entered, are you done?")
                .default(true)
                .interact_on(&Term::stdout()).unwrap();
            if done { break; }
        } else {
            match bip39::Mnemonic::from_word(dic, word.as_str()) {
                Ok(mne) => { mnemonic = Some(mne); break; },
                Err(err) => {
                    println!("Invalid mnemonic: {}", err);
                }
            }
        }
    }

    mnemonic
}

pub fn display_mnemonic_phrase(mnemonic: &bip39::MnemonicString) {
    println!("Note the following words carefully as you will need it to recover your wallet.");
    println!("Press `Enter' when you are sure you have saved them.");
    let prompt = format!("mnemonic: {}", mnemonic);
    while ! Confirmation::new(&prompt).default(true).interact_on(&Term::stdout()).unwrap() {};
}

pub fn get_mnemonic_words<D>(dic: &D) -> bip39::Mnemonics
    where D: bip39::dictionary::Language
{
    let mut vec = vec![];

    println!("Enter the mnemonic word one by one as prompted.");

    for index in 1..25 {
        match get_mnemonic_word(index, dic) {
            None => break,
            Some(idx) => vec.push(idx)
        }
    }

    match bip39::Mnemonics::from_mnemonics(vec) {
        Err(err) => { panic!("Invalid mnemonic phrase: {}", err); },
        Ok(mn) => mn
    }
}

pub fn recover_paperwallet(language: String, opt_pwd: Option<String>) -> bip39::Seed {
    assert!(language == "english");
    let dic = &bip39::dictionary::ENGLISH;

    println!("We are about to recover from a paperwallet. It is the mnemonic words");
    println!("and the password you might have set after generating a new wallet.");

    // 1. get the mnemonic words of the paperwallet
    let shielded_mnemonics = get_mnemonic_words(dic);

    // 2. get the password of the paperwallet
    let pwd = match opt_pwd {
        Some(pwd) => pwd,
        None => get_password()
    };

    // 3. retrieve the shielded entropy
    let shielded_entropy = bip39::Entropy::from_mnemonics(&shielded_mnemonics).unwrap();

    // 4. unscramble the shielded entropy
    let entropy_bytes = paperwallet::unscramble(pwd.as_bytes(), shielded_entropy.as_ref());

    // 5. reconstruct the entropy
    let entropy = bip39::Entropy::from_slice(&entropy_bytes).unwrap();

    // 6. retrieve the mnemonic string
    let mnemonics_str = entropy.to_mnemonics().to_string(dic);

    // 7. rebuild the seed
    bip39::Seed::from_mnemonic_string(&mnemonics_str, pwd.as_bytes())
}

pub fn generate_paper_wallet<D>(dic: &D, entropy: &bip39::Entropy)
    where D: bip39::dictionary::Language
{
    // 1. gen an IV
    let mut iv = [0u8; paperwallet::IV_SIZE];
    for byte in iv.iter_mut() { *byte = rand::random(); }
    println!("We are about to generate a paperwallet. It mainly is a longer mnemonic phrase");
    println!("protected with a password (or not, but un-advised) that you can print and store");
    println!("securely in order to recover your wallet and your funds.");
    // 2. get a password
    let pwd = new_password();
    // 3. generate the scrambled entropy
    let shielded_entropy_bytes = paperwallet::scramble(&iv[..], pwd.as_bytes(), entropy.as_ref());
    // 4. create an entropy from the given bytes
    let shielded_entropy = bip39::Entropy::from_slice(&shielded_entropy_bytes).unwrap();

    println!("shielded entropy: {}",
        shielded_entropy.to_mnemonics().to_string(dic),
    );
}

pub fn generate_entropy(language: String, opt_pwd: Option<String>, t: bip39::Type, no_paper_wallet: bool) -> bip39::Seed {
    assert!(language == "english");
    let dic = &bip39::dictionary::ENGLISH;

    let pwd = match opt_pwd {
        Some(pwd) => pwd,
        None => new_password()
    };

    let entropy = bip39::Entropy::generate(t, rand::random);

    let mnemonic = entropy.to_mnemonics().to_string(dic);
    display_mnemonic_phrase(&mnemonic);

    if ! no_paper_wallet {
        generate_paper_wallet(dic, &entropy);
    }

    bip39::Seed::from_mnemonic_string(&mnemonic, pwd.as_bytes())
}

pub fn recover_entropy(language: String, opt_pwd: Option<String>) -> bip39::Seed {
    assert!(language == "english");
    let dic = &bip39::dictionary::ENGLISH;

    let mnemonics = get_mnemonic_words(dic);

    let pwd = match opt_pwd {
        Some(pwd) => pwd,
        None => get_password()
    };

    let mnemonics_str = mnemonics.to_string(dic);

    bip39::Seed::from_mnemonic_string(&mnemonics_str, pwd.as_bytes())
}

pub fn create_new_account(accounts: &mut config::Accounts, wallet: &config::Config, alias: String) -> wallet::Account {
    let known_accounts : Vec<String> = accounts.iter().filter(|acc| acc.alias.is_some()).map(|acc| acc.alias.clone().unwrap()).collect();
    println!("No account named or indexed {} in your wallet", alias);
    println!("We are about to create a new wallet account.");
    println!("This will allow `{}' to cache some metadata and not require your private keys when", crate_name!());
    println!("performing public operations (like creating addresses).");
    println!("");
    println!("Here is the list of existing accounts: {:?}", known_accounts);

    let prompt = format!("Do you want to create a new account named {:?}?", alias);

    let choice = Confirmation::new(&prompt).default(true).interact_on(&Term::stdout()).unwrap();
    if ! choice { ::std::process::exit(0)}

    accounts.new_account(&wallet.wallet().unwrap(), Some(alias)).unwrap()
}
