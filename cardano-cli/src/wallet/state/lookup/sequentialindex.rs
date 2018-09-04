use cardano::wallet::{bip44};
use std::collections::BTreeMap;
use cardano::address::ExtendedAddr;

use super::{AddressLookup, Address};
use super::super::{utxo::{UTxO}};

pub const DEFAULT_GAP_LIMIT: u32 = 20;

type Result<T> = bip44::bip44::Result<T>;

pub struct SequentialBip44Lookup {
    // cryptographic wallet
    //
    // downside of needed the bip44's wallet is that we need to decrypt the
    // wallet private key with the password. This is needed because we might need
    // to create new addresses and they need hard derivation (which cannot be
    // done through the public key).
    //
    wallet: bip44::Wallet,
    // all the known expected addresses, that includes
    // all different accounts, and also the next not yet live
    // account's addresses
    expected: BTreeMap<ExtendedAddr, bip44::Addressing>,

    // accounts threshold index for internal and external addresses
    accounts: Vec<[bip44::Index;2]>,

    // gap limit
    gap_limit: u32,
}

fn wallet_get_address(wallet: &bip44::Wallet, addr: &bip44::Addressing) -> ExtendedAddr {
    let xprv = wallet.account(wallet.derivation_scheme(), addr.account.get_scheme_value())
                    .change(wallet.derivation_scheme(), addr.address_type())
                    .index(wallet.derivation_scheme(), addr.index.get_scheme_value());
    let xpub = xprv.public();
    let a = ExtendedAddr::new_simple(*xpub);
    a
}

impl SequentialBip44Lookup {
    pub fn new(wallet: bip44::Wallet) -> Self {
        SequentialBip44Lookup {
            wallet: wallet,
            expected: BTreeMap::new(),
            accounts: Vec::new(),
            gap_limit: DEFAULT_GAP_LIMIT,
        }
    }

    fn mut_generate_from(&mut self, account: &bip44::bip44::Account, change: u32, start: &bip44::Index, nb: u32) -> Result<()> {
        let max = start.incr(nb)?;
        let mut r = *start;
        // generate internal and external addresses
        while r < max {
            let addressing = bip44::Addressing { account: *account, change: change, index: r };
            let addr = wallet_get_address(&self.wallet, &addressing);
            self.expected.insert(addr, addressing);
            r = r.incr(1)?;
        }
        Ok(())
    }

    pub fn prepare_next_account(&mut self) -> Result<()> {
        // generate gap limit number of internal and external addresses in the account
        let account_nb = self.accounts.len() as u32;
        let account = bip44::bip44::Account::new(account_nb)?;
        let start = bip44::Index::new(0)?;
        let n = self.gap_limit;
        self.mut_generate_from(&account, 0, &start, n)?;
        self.mut_generate_from(&account, 1, &start, n)?;
        self.accounts.push([start, start]);
        Ok(())
    }

    // every time we find our address, we check if
    // the threshold for the next windows of address is met,
    // and if so, populate the expected cache with the new addresses and update the new threshold
    pub fn threshold_generate(&mut self, addressing: bip44::Addressing) -> Result<()> {
        if addressing.account.get_account_number() as usize >= self.accounts.len() {
            return Ok(());
        }
        let mut limits = self.accounts[addressing.account.get_account_number() as usize];
        if addressing.change != 0 && addressing.change != 1 {
            return Ok(());
        }
        let lidx = addressing.change as usize;
        let current_threshold = limits[lidx];
        if addressing.index <= current_threshold {
            return Ok(());
        }
        let new_threshold = current_threshold.incr(self.gap_limit)?;
        let gap = self.gap_limit;
        self.mut_generate_from(&addressing.account, addressing.change, &new_threshold, gap)?;
        limits[lidx] = new_threshold;
        Ok(())
    }
}

impl AddressLookup for SequentialBip44Lookup {
    type Error = bip44::bip44::Error;

    fn lookup(&mut self, utxo: UTxO<ExtendedAddr>) -> Result<Option<UTxO<Address>>> {
        let addressing = self.expected.get(&utxo.credited_address).cloned();
        if let Some(addressing) = addressing {
            self.threshold_generate(addressing)?;

            Ok(Some(utxo.map(|_| addressing.into())))
        } else { Ok(None) }
    }

    fn acknowledge<A: Into<Address>>(&mut self, address: A) -> Result<()> {
        match address.into() {
            Address::Bip44(address) => self.threshold_generate(address),
            _ => {
                error!("unsupported address (expected bip44 addressing)");
                Err(bip44::bip44::Error::InvalidType(0))
            }
        }
    }
}
