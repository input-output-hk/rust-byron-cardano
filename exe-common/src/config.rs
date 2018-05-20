pub mod net {
    use blockchain::{HeaderHash,EpochId};
    use wallet_crypto::config::{ProtocolMagic};
    use std::{path::{Path}, fs::{File}};
    use serde_yaml;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Config {
        pub domain: String,
        pub genesis: HeaderHash,
        pub genesis_prev: HeaderHash,
        pub protocol_magic: ProtocolMagic,
        pub epoch_start: EpochId,
    }
    impl Config {
        pub fn mainnet() -> Self {
            Config {
                domain: "relays.cardano-mainnet.iohk.io:3000".to_string(),
                genesis: HeaderHash::from_hex(&"89D9B5A5B8DDC8D7E5A6795E9774D97FAF1EFEA59B2CAF7EAF9F8C5B32059DF4").unwrap(),
                genesis_prev: HeaderHash::from_hex(&"5f20df933584822601f9e3f8c024eb5eb252fe8cefb24d1317dc3d432e940ebb").unwrap(),
                protocol_magic: ProtocolMagic::default(),
                epoch_start: 0,
            }
        }

        pub fn testnet() -> Self {
            Config {
                domain: "relays.awstest.iohkdev.io:3000".to_string(),
                genesis: HeaderHash::from_hex(&"B365F1BE6863B453F12B93E1810909B10C79A95EE44BF53414888513FE172C90").unwrap(),
                genesis_prev: HeaderHash::from_hex(&"c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323").unwrap(),
                protocol_magic: ProtocolMagic::new(633343913),
                epoch_start: 0,
            }
        }

        pub fn from_file<P: AsRef<Path>>(p: P) -> Option<Self> {
            let path = p.as_ref();
            if ! path.is_file() {
                return None;
            }

            let mut file = File::open(path).unwrap();
            serde_yaml::from_reader(&mut file).unwrap()
        }
        pub fn to_file<P: AsRef<Path>>(&self, p: P) {
            let mut file = File::create(p.as_ref()).unwrap();
            serde_yaml::to_writer(&mut file, &self).unwrap();
        }
    }
}
