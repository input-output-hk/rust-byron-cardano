use cardano::{bip39, paperwallet, wallet};
use rand;

use termion::{style, color, clear, cursor};
use termion::input::TermRead;
use std::io::{Write, stdout, stdin};

use super::config;

pub fn get_password() -> String {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    stdout.write_all(b"password: ").unwrap();
    stdout.flush().unwrap();

    let pwd = stdin.read_passwd(&mut stdout).unwrap().unwrap_or("".to_string());
    stdout.write_all(b"\n").unwrap();
    stdout.flush().unwrap();
    pwd
}

pub fn new_password() -> String {
    {
        let stdout = stdout();
        let mut stdout = stdout.lock();

        write!(  stdout, "{}", style::Italic).unwrap();
        writeln!(stdout, "Enter you wallet password. It will be needed to recover").unwrap();
        writeln!(stdout, "your wallet later with the mnemonic phrase.").unwrap();
        write!(stdout, "{}", style::NoItalic).unwrap();
        stdout.flush().unwrap();
    }
    let pwd1 = get_password();
    {
        let stdout = stdout();
        let mut stdout = stdout.lock();

        write!(  stdout, "{}", style::Italic).unwrap();
        writeln!(stdout, "Type your password again.").unwrap();
        write!(stdout, "{}", style::NoItalic).unwrap();
        stdout.flush().unwrap();
    }
    let pwd2 = get_password();

    if pwd1 != pwd2 {
        eprintln!("{}not the same password{}", color::Fg(color::Red), color::Fg(color::Reset));
        panic!("try again");
    }

    pwd1
}

pub fn get_mnemonic_word<D>(index: usize, dic: &D) -> Option<bip39::Mnemonic>
    where D: bip39::dictionary::Language
{
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    let mut mmne = None;

    for _ in 0..3 {
        write!(stdout, "mnemonic {}: ", index).unwrap();
        stdout.flush().unwrap();
        let midx = stdin.read_passwd(&mut stdout).unwrap();
        write!(stdout, "{}{}", clear::CurrentLine, cursor::Left(14)).unwrap();
        stdout.flush().unwrap();
        match midx.and_then(|s| if s == "" { None } else { Some(s)}) {
            None => {
                write!(stdout, "{}No mnemonic entered.{} Are you done? (No|yes): ", color::Fg(color::Red), color::Fg(color::Reset)).unwrap();
                stdout.flush().unwrap();
                let mchoice = stdin.read_line().unwrap();
                match mchoice {
                    None => {},
                    Some(choice) => {
                        if choice.to_uppercase() == "YES" { break; }
                    }
                };
            },
            Some(word) => {
                match bip39::Mnemonic::from_word(dic, word.as_str()) {
                    Ok(mne) => { mmne = Some(mne); break; },
                    Err(err) => {
                        writeln!(stdout, "{}Invalid mnemonic{}: {}", color::Fg(color::Red), color::Fg(color::Reset), err).unwrap();
                        stdout.flush().unwrap();
                    }
                }
            }
        }
    }

    mmne
}

pub fn display_mnemonic_phrase(mnemonic: &bip39::MnemonicString) {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let stdin = stdin();
    let mut stdin = stdin.lock();

    write!(  stdout, "{}", style::Italic).unwrap();
    writeln!(stdout, "Note the following words carrefully as you will need it to recover your wallet.").unwrap();
    writeln!(stdout, "Press `Enter' when you are sure you have saved them.").unwrap();
    writeln!(stdout, "{}", style::NoItalic).unwrap();
    write!(stdout, "mnemonic: {}{}{}", color::Fg(color::Green), mnemonic, color::Fg(color::Reset)).unwrap();
    stdout.flush().unwrap();
    let _ = stdin.read_passwd(&mut stdout).unwrap().unwrap();
    write!(stdout, "{}{}", clear::CurrentLine, cursor::Left(128)).unwrap();
    stdout.flush().unwrap();
}

pub fn get_mnemonic_words<D>(dic: &D) -> bip39::Mnemonics
    where D: bip39::dictionary::Language
{
    let mut vec = vec![];

    print!("{}", style::Italic);
    println!("Enter the mnemonic word one by one as prompted.");
    print!("{}", style::NoItalic);

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

    println!("{}", style::Italic);
    println!("We are about to recover from a paperwallet. It is the mnemonic words");
    println!("and the password you might have set after generating a new wallet.");
    println!("{}", style::NoItalic);

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
    println!("{}", style::Italic);
    println!("We are about to generate a paperwallet. It mainly is a longer mnemonic phrase");
    println!("protected with a password (or not, but un-advised) that you can print and store");
    println!("securely in order to recover your wallet and your funds.");
    println!("{}", style::NoItalic);
    // 2. get a password
    let pwd = new_password();
    // 3. generate the scrambled entropy
    let shielded_entropy_bytes = paperwallet::scramble(&iv[..], pwd.as_bytes(), entropy.as_ref());
    // 4. create an antropy from the given bytes
    let shielded_entropy = bip39::Entropy::from_slice(&shielded_entropy_bytes).unwrap();

    println!("shielded entropy: {}{}{}{}{}",
        color::Fg(color::Cyan),
        style::Bold,
        shielded_entropy.to_mnemonics().to_string(dic),
        style::NoBold,
        color::Fg(color::Reset),
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
    println!("{}", style::Italic);
    println!("{}No account named or indexed {} in your wallet{}", color::Fg(color::Red), alias, color::Fg(color::Reset));
    println!("We are about to create a new wallet account.");
    println!("This will allow `{}' to cache some metadata and not require your private keys when", crate_name!());
    println!("performing public operations (like creating addresses).");
    println!("{}", style::NoItalic);
    println!("");
    println!("Here is the list of existing accounts: {:?}", known_accounts);

    {
        let stdout = stdout();
        let mut stdout = stdout.lock();
        let stdin = stdin();
        let mut stdin = stdin.lock();

        write!(stdout, "{}Do you want to create a new account named {:?}?{} (No|yes): ", color::Fg(color::Green), alias, color::Fg(color::Reset)).unwrap();
        stdout.flush().unwrap();
        let mchoice = stdin.read_line().unwrap();
        match mchoice {
            None => { error!("invalid input"); ::std::process::exit(1); },
            Some(choice) => {
                if choice.to_uppercase() == "YES" { ; }
                else { ::std::process::exit(0); }
            }
        };
    }

    accounts.new_account(&wallet.wallet().unwrap(), Some(alias)).unwrap()
}
