use cardano::{bip::bip39::{self, dictionary::Language}};
use console::{style};
use dialoguer::{Input, Confirmation};
use super::super::term::{Term};

fn interactive_input_word<D>(term: &mut Term, dic: &D, idx: usize, count: usize) -> String
    where D: Language
{
    loop {
        let word = Input::new(&format!("mnemonic [{}/{}]", style(idx).cyan(), style(count).cyan().bold()))
            .clear(true)
            .interact_on(&term.term)
            .unwrap();

        match dic.lookup_mnemonic(&word) {
            Ok(_) => return word,
            Err(bip39::dictionary::Error::MnemonicWordNotFoundInDictionary(_)) => {
                let prompt = format!("`{}' is not a valid mnemonic word in `{}'", style(word).italic().red(), style(dic.name()).bold().white());
                while ! Confirmation::new(&prompt).clear(true).default(true).show_default(true).interact_on(&term.term).unwrap() {}
            }
        }
    }
}

type PromptedMnemonics = (bip39::MnemonicString, bip39::Mnemonics, bip39::Entropy);


fn process_mnemonics<D>( dic: &D
                       , string: String
                       )
    -> bip39::Result<PromptedMnemonics>
        where D: Language
{
    let string = bip39::MnemonicString::new(dic, string)?;
    let mnemonics = bip39::Mnemonics::from_string(dic, &string)?;
    let entropy = bip39::Entropy::from_mnemonics(&mnemonics)?;

    Ok((string, mnemonics, entropy))
}

fn validate_mnemonics<D>( dic: &D
                        , size: bip39::Type
                        , string: String
                        )
    -> Result<PromptedMnemonics, String>
        where D: Language
{
    match process_mnemonics(dic, string) {
        Err(err) => {
            debug!("error while processing mnemonics: {}", err);
            match err {
                bip39::Error::InvalidChecksum(_, _) => {
                    let prompt = String::from("Invalid mnemonics (checksum mismatch)");
                    Err(prompt)
                },
                _ => {
                    let prompt = format!("Invalid mnemonics for language `{}'", style(dic.name()).bold().white());
                    Err(prompt)
                }
            }
        },
        Ok(res) => {
            let entered_type = res.1.get_type();
            if size != entered_type {
                let prompt = format!("Invalid mnemonics length. Expected {} mnemonics but received {}.", style(size).bold().white(), style(entered_type).red().bold());
                Err(prompt)
            } else {
                Ok(res)
            }
        },
    }
}

pub fn interactive_input_words<D>(term: &mut Term, dic: &D, size: bip39::Type) -> PromptedMnemonics
    where D: Language
{
    let count = size.mnemonic_count();

    loop {
        let mut string = String::new();
        for idx in 1..=count {
            let result = interactive_input_word(term, dic, idx, count);
            if idx == 1 {
                string = result;
            } else {
                string.push_str(dic.separator());
                string.push_str(&result);
            }
        }

        match validate_mnemonics(dic, size, string) {
            Ok(res) => { return res; },
            Err(prompt) => {
                while ! Confirmation::new(&prompt).clear(true).default(true).show_default(true).interact_on(&term.term).unwrap() {}
            }
        }
    }
}

pub fn input_mnemonic_phrase<D>(term: &mut Term, dic: &D, size: bip39::Type) -> PromptedMnemonics
    where D: Language
{
    let count = size.mnemonic_count();

    loop {
        let string = Input::new(&format!("Please enter all your {} mnemonics", style(count).bold().red()))
            .clear(true)
            .interact_on(&term.term)
            .unwrap();

        match validate_mnemonics(dic, size, string) {
            Ok(res) => { return res; },
            Err(prompt) => {
                while ! Confirmation::new(&prompt).clear(true).default(true).show_default(true).interact_on(&term.term).unwrap() {}
            }
        }
    }
}
