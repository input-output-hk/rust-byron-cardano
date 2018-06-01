use wallet_crypto::bip44;
use wallet_crypto::hdwallet;
use wallet_crypto::wallet::Wallet;
use std::collections::BTreeMap;
use wallet_crypto::address::ExtendedAddr;
use wallet_crypto::tx::{TxIn, TxId, TxOut};
use super::lookup::{AddrLookup, Result, WalletAddr, StatePtr, Utxo, Utxos};
use super::super::config::account;

#[derive(Clone,Debug)]
pub struct SequentialBip44Lookup {
    // cryptographic wallet
    wallet: Wallet,
    // all the known expected addresses, that includes
    // all different accounts, and also the next not yet live
    // account's addresses
    expected: BTreeMap<ExtendedAddr, bip44::Addressing>,

    // accounts threshold index for internal and external addresses
    accounts: Vec<[bip44::Index;2]>,

    // gap limit
    gap_limit: u32,
}

fn wallet_get_address(wallet: &Wallet, addr: &bip44::Addressing) -> ExtendedAddr {
    let xprv = wallet.get_xprv(&addr);
    let xpub = xprv.public();
    let a = ExtendedAddr::new_simple(xpub);
    a
}

impl SequentialBip44Lookup {
    pub fn new(wallet: Wallet) -> Self {
        SequentialBip44Lookup {
            wallet: wallet,
            expected: BTreeMap::new(),
            accounts: Vec::new(),
            gap_limit: 20,
        }
    }

    fn mut_generate_from(&mut self, account: &bip44::Account, change: u32, start: &bip44::Index, nb: u32) -> Result<()> {
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
        let account = bip44::Account::new(account_nb)?;
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

impl AddrLookup for SequentialBip44Lookup {
    fn lookup(&mut self, ptr: &StatePtr, outs: &[(TxId, u32, &TxOut)]) -> Result<Vec<Utxo>> {
        let mut found = Vec::new();
        for o in outs {
            let addressing = self.expected.get(&o.2.address).and_then(|a| Some(a.clone()));
            match addressing {
                None => {},
                Some(addressing) => {
                    // check if we need to generate next window of addresses
                    self.threshold_generate(addressing)?;
                    // found an address from our expected set, so append the txout as ours
                    let utxo = Utxo {
                        block_addr: ptr.clone(),
                        wallet_addr: WalletAddr::Bip44(addressing),
                        txin: TxIn { id: o.0.clone(), index: o.1 },
                        coin: o.2.value,
                    };
                    found.push(utxo)
                },
            }
        }
        Ok(found)
    }

    fn acknowledge_address(&mut self, addr: &WalletAddr) -> Result<()> {
        match addr {
            WalletAddr::Bip44(ref addressing) => self.threshold_generate(addressing.clone()),
            _ => Ok(())
        }
    }
}
