use wallet_crypto::{wallet, bip39};

#[derive(Debug, Serialize, Deserialize)]
pub struct Wallet(pub wallet::Wallet);
impl Wallet {
    pub fn generate(seed: bip39::Seed) -> Self {
        Wallet(wallet::Wallet::new_from_bip39(&seed))
    }
}

