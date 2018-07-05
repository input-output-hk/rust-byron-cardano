//! BIP39 mnemonics
//!
//! Can be used to generate the root key of a given HDWallet,
//! an address or simply convert bits to mnemonic for human friendly
//! value.
//!
//! For more details about the protocol, see Bitcoin Improvment Proposal 39.
//!
//! # Examples
//!
//! ```
//! extern crate rand;
//! extern crate cardano;
//! use cardano::bip39::{Entropy, MnemonicString, Type::{*}, dictionary};
//! use rand::{random};
//!
//! let entropy = Entropy::generate(Type12Words, || random());
//!
//! let mnemonic_phrase = entropy.to_mnemonics().to_string(&dictionary::ENGLISH);
//! ```

use cryptoxide::hmac::{Hmac};
use cryptoxide::sha2::{Sha512};
use cryptoxide::pbkdf2::{pbkdf2};
use std::{fmt, result, str};
use util::{hex};

pub enum Error {
    WrongNumberOfWords(usize),
    WrongKeySize(usize),
    MnemonicOutOfBounf(u16),
    LanguageError(dictionary::Error),
    InvalidSeedSize(usize),
    InvalidChecksum(u8, u8)
}
impl fmt::Debug for Error {
    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self) }
}
impl fmt::Display for Error {
    fn fmt(&self, f:&mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidSeedSize(sz) => {
               write!(f, "Invalid Seed Size, expected {} bytes, but received {} bytes.", SEED_SIZE, sz)
            },
            &Error::WrongNumberOfWords(sz) => {
                write!(f, "Unsupported number of mnemonic words: {}", sz)
            },
            &Error::WrongKeySize(sz) => {
                write!(f, "Unsupported mnemonic entropy size: {}", sz)
            },
            &Error::MnemonicOutOfBounf(val) => {
                write!(f, "The given mnemonic is out of bound, {}", val)
            },
            &Error::LanguageError(ref err) => {
                write!(f, "Mnemonic Dictionary error: {}", err)
            },
            &Error::InvalidChecksum(cs1, cs2) => {
                write!(f, "Invalid Entropy's Checksum, expected {:08b} but found {:08b}", cs1, cs2)
            },
        }
    }
}
impl From<dictionary::Error> for Error {
    fn from(e: dictionary::Error) -> Self { Error::LanguageError(e) }
}

pub type Result<T> = result::Result<T, Error>;

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum Entropy {
    Entropy12([u8;16]),
    Entropy15([u8;20]),
    Entropy18([u8;24]),
    Entropy21([u8;28]),
    Entropy24([u8;32]),
}
impl Entropy {
    pub fn entropy12(bytes: [u8;16]) -> Self { Entropy::Entropy12(bytes) }
    pub fn entropy15(bytes: [u8;20]) -> Self { Entropy::Entropy15(bytes) }
    pub fn entropy18(bytes: [u8;24]) -> Self { Entropy::Entropy18(bytes) }
    pub fn entropy21(bytes: [u8;28]) -> Self { Entropy::Entropy21(bytes) }
    pub fn entropy24(bytes: [u8;32]) -> Self { Entropy::Entropy24(bytes) }

    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let t = Type::from_key_size(bytes.len() * 8)?;
        Ok(Self::new(t, bytes))
    }

    pub fn generate<G>(t: Type, gen: G) -> Self
        where G: Fn() -> u8
    {
        let bytes = [0u8;32];
        let mut entropy = Self::new(t, &bytes[..]);
        for e in entropy.as_mut().iter_mut() { *e = gen(); }
        entropy
    }

    fn new(t: Type, bytes: &[u8]) -> Self {
        let mut e = match t {
            Type::Type12Words => Entropy::Entropy12([0u8;16]),
            Type::Type15Words => Entropy::Entropy15([0u8;20]),
            Type::Type18Words => Entropy::Entropy18([0u8;24]),
            Type::Type21Words => Entropy::Entropy21([0u8;28]),
            Type::Type24Words => Entropy::Entropy24([0u8;32]),
        };
        for i in 0..e.as_ref().len() {
            e.as_mut()[i] = bytes[i]
        };
        e
    }

    pub fn get_type(&self) -> Type {
        match self {
            &Entropy::Entropy12(_) => Type::Type12Words,
            &Entropy::Entropy15(_) => Type::Type15Words,
            &Entropy::Entropy18(_) => Type::Type18Words,
            &Entropy::Entropy21(_) => Type::Type21Words,
            &Entropy::Entropy24(_) => Type::Type24Words,
        }

    }

    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            &mut Entropy::Entropy12(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy15(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy18(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy21(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy24(ref mut b) => b.as_mut(),
        }
    }

    fn hash(&self) -> [u8;32] {
        use cryptoxide::digest::Digest;
        use cryptoxide::sha2::Sha256;
        let mut hasher = Sha256::new();
        let mut res = [0u8;32];
        hasher.input(self.as_ref());
        hasher.result(&mut res);
        res
    }

    pub fn checksum(&self) -> u8 {
        let hash = self.hash()[0];
        match self.get_type() {
            Type::Type12Words => (hash >> 4) & 0b0000_1111,
            Type::Type15Words => (hash >> 3) & 0b0001_1111,
            Type::Type18Words => (hash >> 2) & 0b0011_1111,
            Type::Type21Words => (hash >> 1) & 0b0111_1111,
            Type::Type24Words =>  hash,
        }
    }

    pub fn from_mnemonics(mnemonics: &Mnemonics) -> Result<Self> {
        use util::bits::BitWriterBy11;
        let t = mnemonics.get_type();

        let mut to_validate = BitWriterBy11::new();
        for mnemonic in mnemonics.0.iter() {
            to_validate.write(mnemonic.0);
        }

        let mut r = to_validate.to_bytes();

        let entropy_bytes = Vec::from(&r[..t.to_key_size()/8]);
        let entropy = Self::new(t, &entropy_bytes[..]);
        if let Some(h) = r.pop() {
            let h2 = h >> (8 - t.checksum_size_bits());
            let cs = entropy.checksum();
            if cs != h2 {
                return Err(Error::InvalidChecksum(cs, h2));
            }
        };

        Ok(entropy)
    }

    pub fn to_mnemonics(&self) -> Mnemonics {
        use util::bits::BitReaderBy11;

        let t = self.get_type();
        let mut combined = Vec::from(self.as_ref());
        combined.extend(&self.hash()[..]);

        let mut reader = BitReaderBy11::new(&combined);

        let mut words: Vec<Mnemonic> = Vec::new();
        for _ in 0..t.mnemonic_count() {
            // here we are confident the entropy has already
            // enough bytes to read all the bits we need.
            let n = reader.read();
            // here we can unwrap safely as 11bits can
            // only store up to the value 2047
            words.push(Mnemonic::new(n).unwrap());
        }

        Mnemonics::from_mnemonics(words).unwrap()
    }
}
impl fmt::Display for Entropy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl fmt::Debug for Entropy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl AsRef<[u8]> for Entropy {
    fn as_ref(&self) -> &[u8] {
        match self {
            &Entropy::Entropy12(ref b) => b.as_ref(),
            &Entropy::Entropy15(ref b) => b.as_ref(),
            &Entropy::Entropy18(ref b) => b.as_ref(),
            &Entropy::Entropy21(ref b) => b.as_ref(),
            &Entropy::Entropy24(ref b) => b.as_ref(),
        }
    }
}

pub const SEED_SIZE : usize = 64;
pub struct Seed([u8;64]);
impl Seed {
    /// create a Seed by taking ownership of the given array
    ///
    /// ```
    /// use cardano::bip39::{Seed, SEED_SIZE};
    ///
    /// let bytes = [0u8;SEED_SIZE];
    /// let seed  = Seed::from_bytes(bytes);
    ///
    /// assert!(seed.as_ref().len() == SEED_SIZE);
    /// ```
    pub fn from_bytes(buf: [u8;SEED_SIZE]) -> Self { Seed(buf) }

    /// create a Seed by copying the given slice into a new array
    ///
    /// ```
    /// use cardano::bip39::{Seed, SEED_SIZE};
    ///
    /// let bytes = [0u8;SEED_SIZE];
    /// let wrong = [0u8;31];
    ///
    /// assert!(Seed::from_slice(&wrong[..]).is_err());
    /// assert!(Seed::from_slice(&bytes[..]).is_ok());
    /// ```
    pub fn from_slice(buf: &[u8]) -> Result<Self> {
        if buf.len() != SEED_SIZE {
            return Err(Error::InvalidSeedSize(buf.len()));
        }
        let mut v = [0u8;SEED_SIZE];
        v[..].clone_from_slice(buf);
        Ok(Seed::from_bytes(v))
    }

    pub fn from_mnemonic_string(mnemonics: &MnemonicString, password: &[u8]) -> Self {
        let mut salt = Vec::from("mnemonic".as_bytes());
        salt.extend_from_slice(password);
        let mut mac = Hmac::new(Sha512::new(), mnemonics.0.as_bytes());
        let mut result = [0;SEED_SIZE];
        pbkdf2(&mut mac, &salt, 2048, &mut result);
        Self::from_bytes(result)
    }
}
impl PartialEq for Seed {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref() == other.0.as_ref()
    }
}
impl fmt::Debug for Seed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl fmt::Display for Seed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hex::encode(self.as_ref()))
    }
}
impl AsRef<[u8]> for Seed {
    fn as_ref(&self) -> &[u8] { &self.0 }
}


#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
pub struct MnemonicString(String);
impl MnemonicString {
    pub fn as_str(&self) -> &str { self.0.as_str() }
    pub fn new<D>(dic: &D, s: String) -> Result<Self>
        where D: dictionary::Language
    {
        let _ = Mnemonics::from_string(dic, s.as_str())?;

        Ok(MnemonicString(s))
    }
}
impl fmt::Display for MnemonicString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize, Deserialize)]
pub enum Type {
    Type12Words,
    Type15Words,
    Type18Words,
    Type21Words,
    Type24Words,
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Type::Type12Words => write!(f, "12"),
            &Type::Type15Words => write!(f, "15"),
            &Type::Type18Words => write!(f, "18"),
            &Type::Type21Words => write!(f, "21"),
            &Type::Type24Words => write!(f, "24"),
        }
    }
}
impl str::FromStr for Type {
    type Err = &'static str;
    fn from_str(s: &str) -> result::Result<Self, Self::Err> {
        match s {
            "12" => Ok(Type::Type12Words),
            "15" => Ok(Type::Type15Words),
            "18" => Ok(Type::Type18Words),
            "21" => Ok(Type::Type21Words),
            "24" => Ok(Type::Type24Words),
            _          => Err("Unknown bip39 mnemonic size")
        }
    }
}
impl Type {
    pub fn from_word_count(len: usize) -> Result<Self> {
        match len {
            12 => Ok(Type::Type12Words),
            15 => Ok(Type::Type15Words),
            18 => Ok(Type::Type18Words),
            21 => Ok(Type::Type21Words),
            24 => Ok(Type::Type24Words),
            _  => Err(Error::WrongNumberOfWords(len))
        }
    }

    pub fn from_key_size(len: usize) -> Result<Self> {
        match len {
            128 => Ok(Type::Type12Words),
            160 => Ok(Type::Type15Words),
            192 => Ok(Type::Type18Words),
            224 => Ok(Type::Type21Words),
            256 => Ok(Type::Type24Words),
            _  => Err(Error::WrongKeySize(len))
        }
    }

    pub fn to_key_size(&self) -> usize {
        match self {
            &Type::Type12Words => 128,
            &Type::Type15Words => 160,
            &Type::Type18Words => 192,
            &Type::Type21Words => 224,
            &Type::Type24Words => 256,
        }
    }

    pub fn checksum_size_bits(&self) -> usize {
        match self {
            &Type::Type12Words => 4,
            &Type::Type15Words => 5,
            &Type::Type18Words => 6,
            &Type::Type21Words => 7,
            &Type::Type24Words => 8,
        }
    }

    pub fn mnemonic_count(&self) -> usize {
        match self {
            &Type::Type12Words => 12,
            &Type::Type15Words => 15,
            &Type::Type18Words => 18,
            &Type::Type21Words => 21,
            &Type::Type24Words => 24,
        }

    }
}
impl Default for Type {
    fn default() -> Type { Type::Type18Words }
}

pub const MAX_MNEMONIC_VALUE : u16 = 2048;
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct Mnemonic(pub u16);
impl Mnemonic {
    pub fn new(m: u16) -> Result<Self> {
        if m >= MAX_MNEMONIC_VALUE {
            Err(Error::MnemonicOutOfBounf(m))
        } else {
            Ok(Mnemonic(m))
        }
    }

    pub fn to_word<D>(self, dic: &D) -> String
        where D: dictionary::Language
    {
        dic.lookup_word(self).unwrap()
    }

    pub fn from_word<D>(dic: &D, word: &str) -> Result<Self>
        where D: dictionary::Language
    {
        let v = dic.lookup_mnemonic(word)?;
        Ok(v)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Mnemonics(Vec<Mnemonic>);
impl Mnemonics {
    pub fn get_type(&self) -> Type { Type::from_word_count(self.0.len()).unwrap() }

    pub fn to_string<D>(&self, dic: &D) -> MnemonicString
        where D: dictionary::Language
    {
        let mut vec = String::new();
        let mut first = true;
        for m in self.0.iter() {
            if first { first = false; } else { vec.push_str(r" "); }
            vec.push_str(&m.to_word(dic))
        }
        MnemonicString(vec)
    }

    pub fn from_string<D>(dic: &D, mnemonics: &str) -> Result<Self>
        where D: dictionary::Language
    {
        let mut vec = vec![];
        for word in mnemonics.split_whitespace() {
            vec.push(Mnemonic::from_word(dic, word)?);
        }
        Mnemonics::from_mnemonics(vec)
    }

    pub fn from_mnemonics(mnemonics: Vec<Mnemonic>) -> Result<Self> {
        let _ = Type::from_word_count(mnemonics.len())?;
        Ok(Mnemonics(mnemonics))
    }
}

pub mod dictionary {
    use std::{fmt, result};

    use super::{Mnemonic};

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Serialize, Deserialize)]
    pub enum Error {
        MnemonicWordNotFoundInDictionary(String)
    }
    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &Error::MnemonicWordNotFoundInDictionary(ref s) => {
                    write!(f, "Mnemonic word not found in dictionary \"{}\"", s)
                }
            }
        }
    }

    pub type Result<T> = result::Result<T, Error>;

    pub trait Language {
        fn lookup_mnemonic(&self, word: &str) -> Result<Mnemonic>;
        fn lookup_word(&self, mnemonic: Mnemonic) -> Result<String>;
    }

    pub struct English {
        words: [&'static str;2048]
    }

    impl Language for English {
        fn lookup_mnemonic(&self, word: &str) -> Result<Mnemonic> {
            match self.words.binary_search(&word) {
                Err(_) => Err(Error::MnemonicWordNotFoundInDictionary(word.to_string())),
                Ok(v)  => Ok(Mnemonic::new(v as u16).unwrap())
            }
        }
        fn lookup_word(&self, mnemonic: Mnemonic) -> Result<String> {
            Ok( unsafe { self.words.get_unchecked(mnemonic.0 as usize) }).map(|s| String::from(*s))
        }
    }

    pub const ENGLISH : English = English {
        words: include!("bip39_english.txt")
    };
}


#[cfg(test)]
mod test {
    use super::*;
    extern crate rand;
    use self::rand::random;
    use bip39::dictionary::Language;

    #[test]
    fn english_dic() {
        let dic = &dictionary::ENGLISH;

        assert_eq!(dic.lookup_mnemonic("abandon"), Ok(Mnemonic(0)));
        assert_eq!(dic.lookup_mnemonic("crack"),   Ok(Mnemonic(398)));
        assert_eq!(dic.lookup_mnemonic("shell"),   Ok(Mnemonic(1579)));
        assert_eq!(dic.lookup_mnemonic("zoo"),     Ok(Mnemonic(2047)));

        assert_eq!(dic.lookup_word(Mnemonic(0)),    Ok("abandon".to_string()));
        assert_eq!(dic.lookup_word(Mnemonic(398)),  Ok("crack".to_string()));
        assert_eq!(dic.lookup_word(Mnemonic(1579)), Ok("shell".to_string()));
        assert_eq!(dic.lookup_word(Mnemonic(2047)), Ok("zoo".to_string()));
    }

    #[test]
    fn mnemonic_zero() {
        let entropy = Entropy::entropy12([0;16]);
        let mnemonics = entropy.to_mnemonics();
        let entropy2 = Entropy::from_mnemonics(&mnemonics).unwrap();
        assert_eq!(entropy, entropy2);
    }

    #[test]
    fn mnemonic_7f() {
        let entropy = Entropy::entropy12([0x7f;16]);
        let mnemonics = entropy.to_mnemonics();
        let entropy2 = Entropy::from_mnemonics(&mnemonics).unwrap();
        assert_eq!(entropy, entropy2);
    }

    #[test]
    fn from_mnemonic_to_mnemonic() {
        let entropy = Entropy::generate(Type::Type12Words, random);
        let mnemonics = entropy.to_mnemonics();
        let entropy2 = Entropy::from_mnemonics(&mnemonics).unwrap();
        assert_eq!(entropy, entropy2);
    }


    struct TestVector {
        entropy: &'static str,
        mnemonics: &'static str,
        seed: &'static str,
    }

    fn mk_test(test: &'static TestVector) {
        let mnemonics_str = MnemonicString::new(&dictionary::ENGLISH, test.mnemonics.to_owned())
                            .expect("valid mnemonics string");
        let mnemonics_ref = Mnemonics::from_string(&dictionary::ENGLISH, test.mnemonics)
                            .expect("valid mnemonics");
        let entropy_ref = Entropy::from_slice(&hex::decode(test.entropy).unwrap())
                            .expect("decode entropy from hex");
        let seed_ref = Seed::from_slice(&hex::decode(test.seed).unwrap())
                            .expect("decode seed from hex");

        assert!(mnemonics_ref.get_type() == entropy_ref.get_type());

        assert!(entropy_ref.to_mnemonics() == mnemonics_ref);
        assert!(entropy_ref == Entropy::from_mnemonics(&mnemonics_ref).expect("retrieve entropy from mnemonics"));

        assert!(seed_ref == Seed::from_mnemonic_string(&mnemonics_str, b"TREZOR"));
    }

    #[test]
    fn test_vectors() {
        for test in TEST_VECTORS {
            mk_test(test);
        }
    }

    const TEST_VECTORS : &'static [TestVector] = &
        [ TestVector {
            entropy: "00000000000000000000000000000000",
            mnemonics: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
            seed: "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04",
        },TestVector {
            entropy: "7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f",
            mnemonics: "legal winner thank year wave sausage worth useful legal winner thank yellow",
            seed: "2e8905819b8723fe2c1d161860e5ee1830318dbf49a83bd451cfb8440c28bd6fa457fe1296106559a3c80937a1c1069be3a3a5bd381ee6260e8d9739fce1f607",
        },TestVector {
            entropy: "80808080808080808080808080808080",
            mnemonics: "letter advice cage absurd amount doctor acoustic avoid letter advice cage above",
            seed: "d71de856f81a8acc65e6fc851a38d4d7ec216fd0796d0a6827a3ad6ed5511a30fa280f12eb2e47ed2ac03b5c462a0358d18d69fe4f985ec81778c1b370b652a8",
        },TestVector {
            entropy: "ffffffffffffffffffffffffffffffff",
            mnemonics: "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong",
            seed: "ac27495480225222079d7be181583751e86f571027b0497b5b5d11218e0a8a13332572917f0f8e5a589620c6f15b11c61dee327651a14c34e18231052e48c069",
        },TestVector {
            entropy: "000000000000000000000000000000000000000000000000",
            mnemonics: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent",
            seed: "035895f2f481b1b0f01fcf8c289c794660b289981a78f8106447707fdd9666ca06da5a9a565181599b79f53b844d8a71dd9f439c52a3d7b3e8a79c906ac845fa",
        },TestVector {
            entropy: "7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f",
            mnemonics: "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal will",
            seed: "f2b94508732bcbacbcc020faefecfc89feafa6649a5491b8c952cede496c214a0c7b3c392d168748f2d4a612bada0753b52a1c7ac53c1e93abd5c6320b9e95dd",
        },TestVector {
            entropy: "808080808080808080808080808080808080808080808080",
            mnemonics: "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter always",
            seed: "107d7c02a5aa6f38c58083ff74f04c607c2d2c0ecc55501dadd72d025b751bc27fe913ffb796f841c49b1d33b610cf0e91d3aa239027f5e99fe4ce9e5088cd65",
        },TestVector {
            entropy: "ffffffffffffffffffffffffffffffffffffffffffffffff",
            mnemonics: "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo when",
            seed: "0cd6e5d827bb62eb8fc1e262254223817fd068a74b5b449cc2f667c3f1f985a76379b43348d952e2265b4cd129090758b3e3c2c49103b5051aac2eaeb890a528",
        },TestVector {
            entropy: "0000000000000000000000000000000000000000000000000000000000000000",
            mnemonics: "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art",
            seed: "bda85446c68413707090a52022edd26a1c9462295029f2e60cd7c4f2bbd3097170af7a4d73245cafa9c3cca8d561a7c3de6f5d4a10be8ed2a5e608d68f92fcc8",
        },TestVector {
            entropy: "7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f7f",
            mnemonics: "legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth useful legal winner thank year wave sausage worth title",
            seed: "bc09fca1804f7e69da93c2f2028eb238c227f2e9dda30cd63699232578480a4021b146ad717fbb7e451ce9eb835f43620bf5c514db0f8add49f5d121449d3e87",
        },TestVector {
            entropy: "8080808080808080808080808080808080808080808080808080808080808080",
            mnemonics: "letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic avoid letter advice cage absurd amount doctor acoustic bless",
            seed: "c0c519bd0e91a2ed54357d9d1ebef6f5af218a153624cf4f2da911a0ed8f7a09e2ef61af0aca007096df430022f7a2b6fb91661a9589097069720d015e4e982f",
        },TestVector {
            entropy: "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            mnemonics: "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo vote",
            seed: "dd48c104698c30cfe2b6142103248622fb7bb0ff692eebb00089b32d22484e1613912f0a5b694407be899ffd31ed3992c456cdf60f5d4564b8ba3f05a69890ad",
        },TestVector {
            entropy: "9e885d952ad362caeb4efe34a8e91bd2",
            mnemonics: "ozone drill grab fiber curtain grace pudding thank cruise elder eight picnic",
            seed: "274ddc525802f7c828d8ef7ddbcdc5304e87ac3535913611fbbfa986d0c9e5476c91689f9c8a54fd55bd38606aa6a8595ad213d4c9c9f9aca3fb217069a41028",
        },TestVector {
            entropy: "6610b25967cdcca9d59875f5cb50b0ea75433311869e930b",
            mnemonics: "gravity machine north sort system female filter attitude volume fold club stay feature office ecology stable narrow fog",
            seed: "628c3827a8823298ee685db84f55caa34b5cc195a778e52d45f59bcf75aba68e4d7590e101dc414bc1bbd5737666fbbef35d1f1903953b66624f910feef245ac",
        },TestVector {
            entropy: "68a79eaca2324873eacc50cb9c6eca8cc68ea5d936f98787c60c7ebc74e6ce7c",
            mnemonics: "hamster diagram private dutch cause delay private meat slide toddler razor book happy fancy gospel tennis maple dilemma loan word shrug inflict delay length",
            seed: "64c87cde7e12ecf6704ab95bb1408bef047c22db4cc7491c4271d170a1b213d20b385bc1588d9c7b38f1b39d415665b8a9030c9ec653d75e65f847d8fc1fc440",
        },TestVector {
            entropy: "c0ba5a8e914111210f2bd131f3d5e08d",
            mnemonics: "scheme spot photo card baby mountain device kick cradle pact join borrow",
            seed: "ea725895aaae8d4c1cf682c1bfd2d358d52ed9f0f0591131b559e2724bb234fca05aa9c02c57407e04ee9dc3b454aa63fbff483a8b11de949624b9f1831a9612",
        },TestVector {
            entropy: "6d9be1ee6ebd27a258115aad99b7317b9c8d28b6d76431c3",
            mnemonics: "horn tenant knee talent sponsor spell gate clip pulse soap slush warm silver nephew swap uncle crack brave",
            seed: "fd579828af3da1d32544ce4db5c73d53fc8acc4ddb1e3b251a31179cdb71e853c56d2fcb11aed39898ce6c34b10b5382772db8796e52837b54468aeb312cfc3d",
        },TestVector {
            entropy: "9f6a2878b2520799a44ef18bc7df394e7061a224d2c33cd015b157d746869863",
            mnemonics: "panda eyebrow bullet gorilla call smoke muffin taste mesh discover soft ostrich alcohol speed nation flash devote level hobby quick inner drive ghost inside",
            seed: "72be8e052fc4919d2adf28d5306b5474b0069df35b02303de8c1729c9538dbb6fc2d731d5f832193cd9fb6aeecbc469594a70e3dd50811b5067f3b88b28c3e8d",
        },TestVector {
            entropy: "23db8160a31d3e0dca3688ed941adbf3",
            mnemonics: "cat swing flag economy stadium alone churn speed unique patch report train",
            seed: "deb5f45449e615feff5640f2e49f933ff51895de3b4381832b3139941c57b59205a42480c52175b6efcffaa58a2503887c1e8b363a707256bdd2b587b46541f5",
        },TestVector {
            entropy: "8197a4a47f0425faeaa69deebc05ca29c0a5b5cc76ceacc0",
            mnemonics: "light rule cinnamon wrap drastic word pride squirrel upgrade then income fatal apart sustain crack supply proud access",
            seed: "4cbdff1ca2db800fd61cae72a57475fdc6bab03e441fd63f96dabd1f183ef5b782925f00105f318309a7e9c3ea6967c7801e46c8a58082674c860a37b93eda02",
        },TestVector {
            entropy: "066dca1a2bb7e8a1db2832148ce9933eea0f3ac9548d793112d9a95c9407efad",
            mnemonics: "all hour make first leader extend hole alien behind guard gospel lava path output census museum junior mass reopen famous sing advance salt reform",
            seed: "26e975ec644423f4a4c4f4215ef09b4bd7ef924e85d1d17c4cf3f136c2863cf6df0a475045652c57eb5fb41513ca2a2d67722b77e954b4b3fc11f7590449191d",
        },TestVector {
            entropy: "f30f8c1da665478f49b001d94c5fc452",
            mnemonics: "vessel ladder alter error federal sibling chat ability sun glass valve picture",
            seed: "2aaa9242daafcee6aa9d7269f17d4efe271e1b9a529178d7dc139cd18747090bf9d60295d0ce74309a78852a9caadf0af48aae1c6253839624076224374bc63f",
        },TestVector {
            entropy: "c10ec20dc3cd9f652c7fac2f1230f7a3c828389a14392f05",
            mnemonics: "scissors invite lock maple supreme raw rapid void congress muscle digital elegant little brisk hair mango congress clump",
            seed: "7b4a10be9d98e6cba265566db7f136718e1398c71cb581e1b2f464cac1ceedf4f3e274dc270003c670ad8d02c4558b2f8e39edea2775c9e232c7cb798b069e88",
        },TestVector {
            entropy: "f585c11aec520db57dd353c69554b21a89b20fb0650966fa0a9d6f74fd989d8f",
            mnemonics: "void come effort suffer camp survey warrior heavy shoot primary clutch crush open amazing screen patrol group space point ten exist slush involve unfold",
            seed: "01f5bced59dec48e362f2c45b5de68b9fd6c92c6634f44d6d40aab69056506f0e35524a518034ddc1192e1dacd32c1ed3eaa3c3b131c88ed8e7e54c49a5d0998",
        }
    ];
}
