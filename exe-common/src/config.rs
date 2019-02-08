pub mod net {
    use cardano::block::{EpochId, HeaderHash};
    use cardano::config::ProtocolMagic;
    use serde;
    use serde_yaml;
    use std::{
        fmt,
        fs::{self, File},
        ops::{Deref, DerefMut},
        path::Path,
        str::FromStr,
    };
    use storage_units::utils::tmpfile::TmpFile;

    const DEFAULT_EPOCH_STABILITY_DEPTH: usize = 2160;

    /// A blockchain may have multiple Peer of different kind. Here we define the list
    /// of possible kind of peer we may connect to.
    ///
    /// # Kinds
    ///
    /// ## Native
    ///
    /// The `Peer::Native` kinds are the peer implementing the native peer to peer
    /// protocol. While a native peer may be slower to sync the whole blockchain it
    /// provides more functionalities such as being able to send transactions and
    /// beeing able to keep a connection alive to keep new block as they are created.
    ///
    /// ## Http
    ///
    /// Here we expect to connect to [Hermes](https://github.com/input-output-hk/cardano-rust)
    /// server and to be able to fetch specific blocks or specific EPOCH(s) packed. This method
    /// to sync is blazing fast and allows a clean install to download within seconds the whole
    /// blockchain history. However, it is not possible to send transaction via Hermes.
    ///
    /// # Example
    ///
    /// ```
    /// use exe_common::config::net::{Peer};
    ///
    /// let http_peer = Peer::new("http://hermes.iohk.io".to_string());
    /// assert!(http_peer.is_http());
    ///
    /// let native_peer = Peer::new("mainnet.iohk.io".to_string());
    /// assert!(native_peer.is_native());
    /// ```
    ///
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    pub enum Peer {
        Native(String),
        Http(String),
        Ntt(String),
    }
    impl Peer {
        /// analyse the content of the given `addr` and construct the correct kind
        /// of `Peer` accordingly.
        pub fn new(addr: String) -> Self {
            if addr.starts_with(r"http://") || addr.starts_with(r"https://") {
                Peer::http(addr)
            } else if addr.starts_with(r"ntt://") {
                Peer::ntt(addr)
            } else {
                Peer::native(addr)
            }
        }

        /// force constructing a native `Peer`.
        pub fn native(addr: String) -> Self {
            Peer::Native(addr)
        }
        /// force constructing a http `Peer`.
        pub fn http(addr: String) -> Self {
            Peer::Http(addr)
        }
        /// force constructing a http `Peer`.
        pub fn ntt(addr: String) -> Self {
            Peer::Ntt(addr)
        }
        /// return the content of the native peer if the given object is a native peer.
        pub fn get_native(&self) -> Option<&str> {
            match self {
                &Peer::Native(ref addr) => Some(addr.as_ref()),
                _ => None,
            }
        }
        /// return the content of the http peer if the given object is a http peer.
        pub fn get_http(&self) -> Option<&str> {
            match self {
                &Peer::Http(ref addr) => Some(addr.as_ref()),
                _ => None,
            }
        }
        /// return the content of the ntt peer if the given object is a http peer.
        pub fn get_ntt(&self) -> Option<&str> {
            match self {
                &Peer::Ntt(ref addr) => Some(addr.as_ref()),
                _ => None,
            }
        }


        /// get the address, indifferent to whether the `Peer` is a native or
        /// a http `Peer`.
        pub fn get_address(&self) -> &str {
            match self {
                &Peer::Native(ref addr) => addr.as_ref(),
                &Peer::Http(ref addr) => addr.as_ref(),
                &Peer::Ntt(ref addr) => addr.as_ref(),
            }
        }
        /// test if the `Peer` is a native `Peer`.
        pub fn is_native(&self) -> bool {
            self.get_native().is_some()
        }
        /// test if the `Peer` is a http `Peer`.
        pub fn is_http(&self) -> bool {
            self.get_http().is_some()
        }
        /// test if the `Peer` is a http `Peer`.
        pub fn is_ntt(&self) -> bool {
            self.get_ntt().is_some()
        }
    }
    impl fmt::Display for Peer {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &Peer::Native(ref addr) => write!(f, "native: {}", addr),
                &Peer::Http(ref addr) => write!(f, "http: {}", addr),
                &Peer::Ntt(ref addr) => write!(f, "ntt: {}", addr),
            }
        }
    }
    impl serde::Serialize for Peer {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.get_address().serialize(serializer)
        }
    }
    impl<'de> serde::Deserialize<'de> for Peer {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let addr = String::deserialize(deserializer)?;
            Ok(Self::new(addr))
        }
    }

    #[derive(Debug, Clone)]
    pub struct NamedPeer(String, Peer);
    impl NamedPeer {
        pub fn new(name: String, peer: Peer) -> Self {
            NamedPeer(name, peer)
        }
        pub fn name(&self) -> &str {
            self.0.as_str()
        }
        pub fn peer(&self) -> &Peer {
            &self.1
        }
    }
    impl Deref for NamedPeer {
        type Target = Peer;
        fn deref(&self) -> &Self::Target {
            &self.1
        }
    }
    impl DerefMut for NamedPeer {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.1
        }
    }
    impl serde::Serialize for NamedPeer {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            use serde::ser::SerializeMap;
            let mut map_serializer = serializer.serialize_map(Some(1))?;
            map_serializer.serialize_entry(self.name(), self.peer())?;
            map_serializer.end()
        }
    }
    impl<'de> serde::Deserialize<'de> for NamedPeer {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = NamedPeer;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a NamedPeer")
                }

                #[inline]
                fn visit_map<V>(self, mut visitor: V) -> Result<Self::Value, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
                    V::Error: serde::de::Error,
                {
                    if let Some((k, v)) = visitor.next_entry()? {
                        Ok(NamedPeer::new(k, v))
                    } else {
                        Err(serde::de::Error::invalid_length(
                            0,
                            &"one and only one entry",
                        ))
                    }
                }
            }

            deserializer.deserialize_map(Visitor)
        }
    }

    /// collection of named `Peer`.
    ///
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Peers(Vec<NamedPeer>);
    impl Peers {
        /// create an empty collection of peers
        pub fn new() -> Self {
            Peers(Vec::new())
        }

        /// add a new peer in the `Peers` set
        pub fn push(&mut self, name: String, peer: Peer) {
            self.0.push(NamedPeer::new(name, peer))
        }

        pub fn natives<'a>(&'a self) -> Vec<&'a str> {
            self.iter()
                .filter_map(|np| np.peer().get_native())
                .collect()
        }
    }
    impl Deref for Peers {
        type Target = Vec<NamedPeer>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl ::std::iter::FromIterator<NamedPeer> for Peers {
        fn from_iter<I>(iter: I) -> Self
        where
            I: IntoIterator<Item = NamedPeer>,
        {
            Peers(::std::iter::FromIterator::from_iter(iter))
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Config {
        pub genesis: HeaderHash,
        pub genesis_prev: HeaderHash,
        pub epoch_stability_depth: usize,
        pub protocol_magic: ProtocolMagic,
        pub epoch_start: EpochId,
        pub peers: Peers,
    }
    impl Config {
        pub fn mainnet() -> Self {
            let mut peers = Peers::new();
            peers.push(
                "iohk-hosts".to_string(),
                Peer::native("relays.cardano-mainnet.iohk.io:3000".to_string()),
            );
            peers.push(
                "hermes".to_string(),
                Peer::http("http://hermes.dev.iohkdev.io/mainnet".to_string()),
            );
            Config {
                genesis: HeaderHash::from_str(
                    &"89D9B5A5B8DDC8D7E5A6795E9774D97FAF1EFEA59B2CAF7EAF9F8C5B32059DF4",
                )
                .unwrap(),
                genesis_prev: HeaderHash::from_str(
                    &"5f20df933584822601f9e3f8c024eb5eb252fe8cefb24d1317dc3d432e940ebb",
                )
                .unwrap(),
                epoch_stability_depth: DEFAULT_EPOCH_STABILITY_DEPTH,
                protocol_magic: ProtocolMagic::default(),
                epoch_start: 0,
                peers: peers,
            }
        }

        pub fn staging() -> Self {
            let mut peers = Peers::new();
            peers.push(
                "iohk-hosts".to_string(),
                Peer::native("relays.awstest.iohkdev.io:3000".to_string()),
            );
            peers.push(
                "hermes".to_string(),
                Peer::http("http://hermes.dev.iohkdev.io/staging".to_string()),
            );
            Config {
                genesis: HeaderHash::from_str(
                    &"B365F1BE6863B453F12B93E1810909B10C79A95EE44BF53414888513FE172C90",
                )
                .unwrap(),
                genesis_prev: HeaderHash::from_str(
                    &"c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323",
                )
                .unwrap(),
                epoch_stability_depth: DEFAULT_EPOCH_STABILITY_DEPTH,
                protocol_magic: ProtocolMagic::from(633343913),
                epoch_start: 0,
                peers: peers,
            }
        }

        pub fn testnet() -> Self {
            let mut peers = Peers::new();
            peers.push(
                "iohk-hosts".to_string(),
                Peer::native("relays.cardano-testnet.iohkdev.io:3000".to_string()),
            );
            peers.push(
                "hermes".to_string(),
                Peer::http("http://hermes.dev.iohkdev.io/testnet".to_string()),
            );
            Config {
                genesis: HeaderHash::from_str(
                    &"7f141ea26e189c9cb09e2473f6499561011d5d3c90dd642fde859ce02282a3ae",
                )
                .unwrap(),
                genesis_prev: HeaderHash::from_str(
                    &"b7f76950bc4866423538ab7764fc1c7020b24a5f717a5bee3109ff2796567214",
                )
                .unwrap(),
                epoch_start: 0,
                epoch_stability_depth: DEFAULT_EPOCH_STABILITY_DEPTH,
                protocol_magic: ProtocolMagic::from(1097911063),
                peers: peers,
            }
        }

        pub fn from_file<P: AsRef<Path>>(p: P) -> Option<Self> {
            let path = p.as_ref();
            if !path.is_file() {
                return None;
            }

            let mut file = File::open(path).unwrap();
            serde_yaml::from_reader(&mut file).unwrap()
        }
        pub fn to_file<P: AsRef<Path>>(&self, p: P) {
            let dir = p.as_ref().parent().unwrap().to_path_buf();
            fs::DirBuilder::new()
                .recursive(true)
                .create(dir.clone())
                .unwrap();
            let mut file = TmpFile::create(dir).unwrap();
            serde_yaml::to_writer(&mut file, &self).unwrap();
            file.render_permanent(&p.as_ref().to_path_buf()).unwrap();
        }
    }
}
