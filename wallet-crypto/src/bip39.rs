use rcw::hmac::{Hmac};
use rcw::sha2::{Sha512};
use rcw::pbkdf2::{pbkdf2};
use std::{fmt, result, str};
use util::{hex};
use bit_vec::{BitVec};
use bitreader::{BitReader};

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
        use rcw::digest::Digest;
        use rcw::sha2::Sha256;
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
        let t = mnemonics.get_type();

        fn bit_from_u16_as_u11(input: u16, position: u16) -> bool {
            if position < 11 {
                input & (1 << (10 - position)) != 0
            } else {
                false
            }
        }

        let mut to_validate: BitVec = BitVec::new();
        for mnemonic in mnemonics.0.iter() {
            let n = mnemonic.0;
            for i in 0..11 {
                to_validate.push(bit_from_u16_as_u11(n, i));
            }
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
        let t = self.get_type();
        let mut combined = Vec::from(self.as_ref());
        combined.extend(&self.hash()[..]);

        let mut reader = BitReader::new(&combined);

        let mut words: Vec<Mnemonic> = Vec::new();
        for _ in 0..t.mnemonic_count() {
            // here we are confident the entropy has already
            // enough bytes to read all the bits we need.
            let n = reader.read_u16(11).unwrap();
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
    /// use wallet_crypto::bip39::{Seed, SEED_SIZE};
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
    /// use wallet_crypto::bip39::{Seed, SEED_SIZE};
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
}