//! BIP39 mnemonics
//!
//! Can be used to generate the root key of a given HDWallet,
//! an address or simply convert bits to mnemonic for human friendly
//! value.
//!
//! For more details about the protocol, see
//! [Bitcoin Improvement Proposal 39](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
//!
//! # Example
//!
//! ## To create a new HDWallet
//!
//! ```
//! extern crate cardano;
//! extern crate rand;
//!
//! use cardano::bip::bip39::*;
//!
//! // first, you need to generate the original entropy
//! let entropy = Entropy::generate(Type::Type18Words, rand::random);
//!
//! // human readable mnemonics (in English) to retrieve the original entropy
//! // and eventually recover a HDWallet.
//! let mnemonic_phrase = entropy.to_mnemonics().to_string(&dictionary::ENGLISH);
//!
//! // The seed of the HDWallet is generated from the mnemonic string
//! // in the associated language.
//! let seed = Seed::from_mnemonic_string(&mnemonic_phrase, b"some password");
//! ```
//!
//! ## To recover a HDWallet
//!
//! ```
//! use cardano::bip::bip39::*;
//!
//! let mnemonics = "mimic left ask vacant toast follow bitter join diamond gate attend obey";
//!
//! // to retrieve the seed, you only need the mnemonic string,
//! // here we construct the `MnemonicString` by verifying the
//! // mnemonics are valid against the given dictionary (English here).
//! let mnemonic_phrase = MnemonicString::new(&dictionary::ENGLISH, mnemonics.to_owned())
//!     .expect("the given mnemonics are valid English words");
//!
//! // The seed of the HDWallet is generated from the mnemonic string
//! // in the associated language.
//! let seed = Seed::from_mnemonic_string(&mnemonic_phrase, b"some password");
//! ```
//!

use cryptoxide::hmac::Hmac;
use cryptoxide::pbkdf2::pbkdf2;
use cryptoxide::sha2::Sha512;
use std::{error, fmt, ops::Deref, result, str};
use util::{hex, securemem};

/// Error regarding BIP39 operations
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// Received an unsupported number of mnemonic words. The parameter
    /// contains the unsupported number. Supported values are
    /// described as part of the [`Type`](./enum.Type.html).
    WrongNumberOfWords(usize),

    /// The entropy is of invalid size. The parameter contains the invalid size,
    /// the list of supported entropy size are described as part of the
    /// [`Type`](./enum.Type.html).
    WrongKeySize(usize),

    /// The given mnemonic is out of bound, i.e. its index is above 2048 and
    /// is invalid within BIP39 specifications.
    MnemonicOutOfBound(u16),

    /// Forward error regarding dictionary operations.
    LanguageError(dictionary::Error),

    /// the Seed is of invalid size. The parameter is the given seed size,
    /// the expected seed size is [`SEED_SIZE`](./constant.SEED_SIZE.html).
    InvalidSeedSize(usize),

    /// checksum is invalid. The first parameter is the expected checksum,
    /// the second id the computed checksum. This error means that the given
    /// mnemonics are invalid to retrieve the original entropy. The user might
    /// have given an invalid mnemonic phrase.
    InvalidChecksum(u8, u8),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Error::InvalidSeedSize(sz) => write!(
                f,
                "Invalid Seed Size, expected {} bytes, but received {} bytes.",
                SEED_SIZE, sz
            ),
            &Error::WrongNumberOfWords(sz) => {
                write!(f, "Unsupported number of mnemonic words: {}", sz)
            }
            &Error::WrongKeySize(sz) => write!(f, "Unsupported mnemonic entropy size: {}", sz),
            &Error::MnemonicOutOfBound(val) => {
                write!(f, "The given mnemonic is out of bound, {}", val)
            }
            &Error::LanguageError(_) => write!(f, "Unknown mnemonic word"),
            &Error::InvalidChecksum(cs1, cs2) => write!(
                f,
                "Invalid Entropy's Checksum, expected {:08b} but found {:08b}",
                cs1, cs2
            ),
        }
    }
}
impl From<dictionary::Error> for Error {
    fn from(e: dictionary::Error) -> Self {
        Error::LanguageError(e)
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&error::Error> {
        match self {
            Error::LanguageError(ref error) => Some(error),
            _ => None,
        }
    }
}

/// convenient Alias to wrap up BIP39 operations that may return
/// an [`Error`](./enum.Error.html).
pub type Result<T> = result::Result<T, Error>;

/// BIP39 entropy is used as root entropy for the HDWallet PRG
/// to generate the HDWallet root keys.
///
/// See module documentation for mode details about how to use
/// `Entropy`.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Entropy {
    Entropy9([u8; 12]),
    Entropy12([u8; 16]),
    Entropy15([u8; 20]),
    Entropy18([u8; 24]),
    Entropy21([u8; 28]),
    Entropy24([u8; 32]),
}
impl Entropy {
    /// Retrieve an `Entropy` from the given slice.
    ///
    /// # Error
    ///
    /// This function may fail if the given slice's length is not
    /// one of the supported entropy length. See [`Type`](./enum.Type.html)
    /// for the list of supported entropy sizes.
    ///
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let t = Type::from_entropy_size(bytes.len() * 8)?;
        Ok(Self::new(t, bytes))
    }

    /// generate entropy using the given random generator.
    ///
    /// # Example
    ///
    /// ```
    /// extern crate rand;
    /// # extern crate cardano;
    /// # use cardano::bip::bip39::*;
    ///
    /// let entropy = Entropy::generate(Type::Type15Words, rand::random);
    /// ```
    ///
    pub fn generate<G>(t: Type, gen: G) -> Self
    where
        G: Fn() -> u8,
    {
        let bytes = [0u8; 32];
        let mut entropy = Self::new(t, &bytes[..]);
        for e in entropy.as_mut().iter_mut() {
            *e = gen();
        }
        entropy
    }

    fn new(t: Type, bytes: &[u8]) -> Self {
        let mut e = match t {
            Type::Type9Words => Entropy::Entropy9([0u8; 12]),
            Type::Type12Words => Entropy::Entropy12([0u8; 16]),
            Type::Type15Words => Entropy::Entropy15([0u8; 20]),
            Type::Type18Words => Entropy::Entropy18([0u8; 24]),
            Type::Type21Words => Entropy::Entropy21([0u8; 28]),
            Type::Type24Words => Entropy::Entropy24([0u8; 32]),
        };
        for i in 0..e.as_ref().len() {
            e.as_mut()[i] = bytes[i]
        }
        e
    }

    /// handy helper to retrieve the [`Type`](./enum.Type.html)
    /// from the `Entropy`.
    #[inline]
    pub fn get_type(&self) -> Type {
        match self {
            &Entropy::Entropy9(_) => Type::Type9Words,
            &Entropy::Entropy12(_) => Type::Type12Words,
            &Entropy::Entropy15(_) => Type::Type15Words,
            &Entropy::Entropy18(_) => Type::Type18Words,
            &Entropy::Entropy21(_) => Type::Type21Words,
            &Entropy::Entropy24(_) => Type::Type24Words,
        }
    }

    fn as_mut(&mut self) -> &mut [u8] {
        match self {
            &mut Entropy::Entropy9(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy12(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy15(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy18(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy21(ref mut b) => b.as_mut(),
            &mut Entropy::Entropy24(ref mut b) => b.as_mut(),
        }
    }

    fn hash(&self) -> [u8; 32] {
        use cryptoxide::digest::Digest;
        use cryptoxide::sha2::Sha256;
        let mut hasher = Sha256::new();
        let mut res = [0u8; 32];
        hasher.input(self.as_ref());
        hasher.result(&mut res);
        res
    }

    /// compute the checksum of the entropy, be aware that only
    /// part of the bytes may be useful for the checksum depending
    /// of the [`Type`](./enum.Type.html) of the `Entropy`.
    ///
    /// | entropy type | checksum size (in bits) |
    /// | ------------ | ----------------------- |
    /// | 9 words      | 3 bits                  |
    /// | 12 words     | 4 bits                  |
    /// | 15 words     | 5 bits                  |
    /// | 18 words     | 6 bits                  |
    /// | 21 words     | 7 bits                  |
    /// | 24 words     | 8 bits                  |
    ///
    /// # Example
    ///
    /// ```
    /// # extern crate rand;
    /// # extern crate cardano;
    /// # use cardano::bip::bip39::*;
    ///
    /// let entropy = Entropy::generate(Type::Type15Words, rand::random);
    ///
    /// let checksum = entropy.checksum() & 0b0001_1111;
    /// ```
    ///
    pub fn checksum(&self) -> u8 {
        let hash = self.hash()[0];
        match self.get_type() {
            Type::Type9Words => (hash >> 5) & 0b0000_0111,
            Type::Type12Words => (hash >> 4) & 0b0000_1111,
            Type::Type15Words => (hash >> 3) & 0b0001_1111,
            Type::Type18Words => (hash >> 2) & 0b0011_1111,
            Type::Type21Words => (hash >> 1) & 0b0111_1111,
            Type::Type24Words => hash,
        }
    }

    /// retrieve the `Entropy` from the given [`Mnemonics`](./struct.Mnemonics.html).
    ///
    /// # Example
    ///
    /// ```
    /// # use cardano::bip::bip39::*;
    ///
    /// const MNEMONICS : &'static str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    /// let mnemonics = Mnemonics::from_string(&dictionary::ENGLISH, MNEMONICS)
    ///     .expect("validating the given mnemonics phrase");
    ///
    /// let entropy = Entropy::from_mnemonics(&mnemonics)
    ///     .expect("retrieving the entropy from the mnemonics");
    /// ```
    ///
    /// # Error
    ///
    /// This function may fail if the Mnemonic has an invalid checksum. As part of the
    /// BIP39, the checksum must be embedded in the mnemonic phrase. This allow to check
    /// the mnemonics have been correctly entered by the user.
    ///
    pub fn from_mnemonics(mnemonics: &Mnemonics) -> Result<Self> {
        use util::bits::BitWriterBy11;
        let t = mnemonics.get_type();

        let mut to_validate = BitWriterBy11::new();
        for mnemonic in mnemonics.0.iter() {
            to_validate.write(mnemonic.0);
        }

        let mut r = to_validate.to_bytes();

        let entropy_bytes = Vec::from(&r[..t.to_key_size() / 8]);
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

    /// convert the given `Entropy` into a mnemonic phrase.
    ///
    /// # Example
    ///
    /// ```
    /// # use cardano::bip::bip39::*;
    ///
    /// let entropy = Entropy::Entropy12([0;16]);
    ///
    /// let mnemonics = entropy.to_mnemonics()
    ///     .to_string(&dictionary::ENGLISH);
    /// ```
    ///
    pub fn to_mnemonics(&self) -> Mnemonics {
        use util::bits::BitReaderBy11;

        let t = self.get_type();
        let mut combined = Vec::from(self.as_ref());
        combined.extend(&self.hash()[..]);

        let mut reader = BitReaderBy11::new(&combined);

        let mut words: Vec<MnemonicIndex> = Vec::new();
        for _ in 0..t.mnemonic_count() {
            // here we are confident the entropy has already
            // enough bytes to read all the bits we need.
            let n = reader.read();
            // assert only in non optimized builds, Since we read 11bits
            // by 11 bits we should not allow values beyond 2047.
            debug_assert!( n <= MAX_MNEMONIC_VALUE
                         , "Something went wrong, the BitReaderBy11 did return an impossible value: {} (0b{:016b})"
                         , n, n
                         );
            // here we can unwrap safely as 11bits can
            // only store up to the value 2047
            words.push(MnemonicIndex::new(n).unwrap());
        }
        // by design, it is safe to call unwrap here as
        // the mnemonic length has been validated by construction.
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
            &Entropy::Entropy9(ref b) => b.as_ref(),
            &Entropy::Entropy12(ref b) => b.as_ref(),
            &Entropy::Entropy15(ref b) => b.as_ref(),
            &Entropy::Entropy18(ref b) => b.as_ref(),
            &Entropy::Entropy21(ref b) => b.as_ref(),
            &Entropy::Entropy24(ref b) => b.as_ref(),
        }
    }
}
impl Deref for Entropy {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl Drop for Entropy {
    fn drop(&mut self) {
        match self {
            Entropy::Entropy9(b) => securemem::zero(b),
            Entropy::Entropy12(b) => securemem::zero(b),
            Entropy::Entropy15(b) => securemem::zero(b),
            Entropy::Entropy18(b) => securemem::zero(b),
            Entropy::Entropy21(b) => securemem::zero(b),
            Entropy::Entropy24(b) => securemem::zero(b),
        }
    }
}

/// the expected size of a seed, in bytes.
pub const SEED_SIZE: usize = 64;

/// A BIP39 `Seed` object, will be used to generate a given HDWallet
/// root key.
///
/// See the module documentation for more details about how to use it
/// within the `cardano` library.
pub struct Seed([u8; SEED_SIZE]);
impl Seed {
    /// create a Seed by taking ownership of the given array
    ///
    /// # Example
    ///
    /// ```
    /// use cardano::bip::bip39::{Seed, SEED_SIZE};
    ///
    /// let bytes = [0u8;SEED_SIZE];
    /// let seed  = Seed::from_bytes(bytes);
    ///
    /// assert!(seed.as_ref().len() == SEED_SIZE);
    /// ```
    pub fn from_bytes(buf: [u8; SEED_SIZE]) -> Self {
        Seed(buf)
    }

    /// create a Seed by copying the given slice into a new array
    ///
    /// # Example
    ///
    /// ```
    /// use cardano::bip::bip39::{Seed, SEED_SIZE};
    ///
    /// let bytes = [0u8;SEED_SIZE];
    /// let wrong = [0u8;31];
    ///
    /// assert!(Seed::from_slice(&wrong[..]).is_err());
    /// assert!(Seed::from_slice(&bytes[..]).is_ok());
    /// ```
    ///
    /// # Error
    ///
    /// This constructor may fail if the given slice's length is not
    /// compatible to define a `Seed` (see [`SEED_SIZE`](./constant.SEED_SIZE.html)).
    ///
    pub fn from_slice(buf: &[u8]) -> Result<Self> {
        if buf.len() != SEED_SIZE {
            return Err(Error::InvalidSeedSize(buf.len()));
        }
        let mut v = [0u8; SEED_SIZE];
        v[..].clone_from_slice(buf);
        Ok(Seed::from_bytes(v))
    }

    /// get the seed from the given [`MnemonicString`] and the given password.
    ///
    /// [`MnemonicString`]: ./struct.MnemonicString.html
    ///
    /// Note that the `Seed` is not generated from the `Entropy` directly. It is a
    /// design choice of Bip39.
    ///
    /// # Safety
    ///
    /// The password is meant to allow plausible deniability. While it is possible
    /// not to use a password to protect the HDWallet it is better to add one.
    ///
    /// # Example
    ///
    /// ```
    /// # use cardano::bip::bip39::*;
    ///
    /// const MNEMONICS : &'static str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    /// let mnemonics = MnemonicString::new(&dictionary::ENGLISH, MNEMONICS.to_owned())
    ///     .expect("valid Mnemonic phrase");
    ///
    /// let seed = Seed::from_mnemonic_string(&mnemonics, b"Bourbaki team rocks!");
    /// ```
    ///
    pub fn from_mnemonic_string(mnemonics: &MnemonicString, password: &[u8]) -> Self {
        let mut salt = Vec::from("mnemonic".as_bytes());
        salt.extend_from_slice(password);
        let mut mac = Hmac::new(Sha512::new(), mnemonics.0.as_bytes());
        let mut result = [0; SEED_SIZE];
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
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
impl Deref for Seed {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}
impl Drop for Seed {
    fn drop(&mut self) {
        self.0.copy_from_slice(&[0; SEED_SIZE][..]);
    }
}

/// RAII for validated mnemonic words. This guarantee a given mnemonic phrase
/// has been safely validated against a dictionary.
///
/// See the module documentation for more details about how to use it
/// within the `cardano` library.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub struct MnemonicString(String);
impl MnemonicString {
    /// create a `MnemonicString` from the given `String`. This function
    /// will validate the mnemonic phrase against the given [`Language`]
    ///
    /// [`Language`]: ./dictionary/trait.Language.html
    ///
    /// # Example
    ///
    /// ```
    /// # use cardano::bip::bip39::*;
    ///
    /// const MNEMONICS : &'static str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    /// let mnemonics = MnemonicString::new(&dictionary::ENGLISH, MNEMONICS.to_owned())
    ///     .expect("valid Mnemonic phrase");
    /// ```
    ///
    /// # Error
    ///
    /// This function may fail if one or all words are not recognized
    /// in the given [`Language`].
    ///
    pub fn new<D>(dic: &D, s: String) -> Result<Self>
    where
        D: dictionary::Language,
    {
        let _ = Mnemonics::from_string(dic, &s)?;

        Ok(MnemonicString(s))
    }
}
impl Deref for MnemonicString {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl fmt::Display for MnemonicString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The support type of `Mnemonics`, i.e. the number of words supported in a
/// mnemonic phrase.
///
/// This enum provide the following properties:
///
/// | number of words | entropy size (bits) | checksum size (bits)  |
/// | --------------- | ------------------- | --------------------- |
/// | 9               | 96                  | 3                     |
/// | 12              | 128                 | 4                     |
/// | 15              | 160                 | 5                     |
/// | 18              | 192                 | 6                     |
/// | 21              | 224                 | 7                     |
/// | 24              | 256                 | 8                     |
///
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
#[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
pub enum Type {
    Type9Words,
    Type12Words,
    Type15Words,
    Type18Words,
    Type21Words,
    Type24Words,
}
impl Type {
    pub fn from_word_count(len: usize) -> Result<Self> {
        match len {
            9 => Ok(Type::Type9Words),
            12 => Ok(Type::Type12Words),
            15 => Ok(Type::Type15Words),
            18 => Ok(Type::Type18Words),
            21 => Ok(Type::Type21Words),
            24 => Ok(Type::Type24Words),
            _ => Err(Error::WrongNumberOfWords(len)),
        }
    }

    pub fn from_entropy_size(len: usize) -> Result<Self> {
        match len {
            96 => Ok(Type::Type9Words),
            128 => Ok(Type::Type12Words),
            160 => Ok(Type::Type15Words),
            192 => Ok(Type::Type18Words),
            224 => Ok(Type::Type21Words),
            256 => Ok(Type::Type24Words),
            _ => Err(Error::WrongKeySize(len)),
        }
    }

    pub fn to_key_size(&self) -> usize {
        match self {
            &Type::Type9Words => 96,
            &Type::Type12Words => 128,
            &Type::Type15Words => 160,
            &Type::Type18Words => 192,
            &Type::Type21Words => 224,
            &Type::Type24Words => 256,
        }
    }

    pub fn checksum_size_bits(&self) -> usize {
        match self {
            &Type::Type9Words => 3,
            &Type::Type12Words => 4,
            &Type::Type15Words => 5,
            &Type::Type18Words => 6,
            &Type::Type21Words => 7,
            &Type::Type24Words => 8,
        }
    }

    pub fn mnemonic_count(&self) -> usize {
        match self {
            &Type::Type9Words => 9,
            &Type::Type12Words => 12,
            &Type::Type15Words => 15,
            &Type::Type18Words => 18,
            &Type::Type21Words => 21,
            &Type::Type24Words => 24,
        }
    }
}
impl Default for Type {
    fn default() -> Type {
        Type::Type18Words
    }
}
impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Type::Type9Words => write!(f, "9"),
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
            "9" => Ok(Type::Type9Words),
            "12" => Ok(Type::Type12Words),
            "15" => Ok(Type::Type15Words),
            "18" => Ok(Type::Type18Words),
            "21" => Ok(Type::Type21Words),
            "24" => Ok(Type::Type24Words),
            _ => Err("Unknown bip39 mnemonic size"),
        }
    }
}

/// the maximum authorized value for a mnemonic. i.e. 2047
pub const MAX_MNEMONIC_VALUE: u16 = 2047;

/// Safe representation of a valid mnemonic index (see
/// [`MAX_MNEMONIC_VALUE`](./constant.MAX_MNEMONIC_VALUE.html)).
///
/// See [`dictionary module documentation`](./dictionary/index.html) for
/// more details about how to use this.
///
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub struct MnemonicIndex(pub u16);

impl MnemonicIndex {
    /// smart constructor, validate the given value fits the mnemonic index
    /// boundaries (see [`MAX_MNEMONIC_VALUE`](./constant.MAX_MNEMONIC_VALUE.html)).
    ///
    /// # Example
    ///
    /// ```
    /// # use cardano::bip::bip39::*;
    /// #
    /// let index = MnemonicIndex::new(1029);
    /// assert!(index.is_ok());
    /// // this line will fail
    /// let index = MnemonicIndex::new(4029);
    /// assert_eq!(index, Err(Error::MnemonicOutOfBound(4029)));
    /// ```
    ///
    /// # Error
    ///
    /// returns an [`Error::MnemonicOutOfBound`](enum.Error.html#variant.MnemonicOutOfBound)
    /// if the given value does not fit the valid values.
    ///
    pub fn new(m: u16) -> Result<Self> {
        if m <= MAX_MNEMONIC_VALUE {
            Ok(MnemonicIndex(m))
        } else {
            Err(Error::MnemonicOutOfBound(m))
        }
    }

    /// lookup in the given dictionary to retrieve the mnemonic word.
    ///
    /// # panic
    ///
    /// this function may panic if the
    /// [`Language::lookup_word`](./dictionary/trait.Language.html#method.lookup_word)
    /// returns an error. Which should not happen.
    ///
    pub fn to_word<D>(self, dic: &D) -> String
    where
        D: dictionary::Language,
    {
        dic.lookup_word(self).unwrap()
    }

    /// retrieve the Mnemonic index from the given word in the
    /// given dictionary.
    ///
    /// # Error
    ///
    /// May fail with a [`LanguageError`](enum.Error.html#variant.LanguageError)
    /// if the given [`Language`](./dictionary/trait.Language.html) returns the
    /// given word is not within its dictionary.
    ///
    pub fn from_word<D>(dic: &D, word: &str) -> Result<Self>
    where
        D: dictionary::Language,
    {
        let v = dic.lookup_mnemonic(word)?;
        Ok(v)
    }
}

/// Language agnostic mnemonic phrase representation.
///
/// This is an handy intermediate representation of a given mnemonic
/// phrase. One can use this intermediate representation to translate
/// mnemonic from one [`Language`](./dictionary/trait.Language.html)
/// to another. **However** keep in mind that the [`Seed`](./struct.Seed.html)
/// is linked to the mnemonic string in a specific language, in a specific
/// dictionary. The [`Entropy`](./struct.Entropy.html) will be the same
/// but the resulted [`Seed`](./struct.Seed.html) will differ and all
/// the derived key of a HDWallet using the [`Seed`](./struct.Seed.html)
/// as a source to generate the root key.
///
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Mnemonics(Vec<MnemonicIndex>);

impl AsRef<[MnemonicIndex]> for Mnemonics {
    fn as_ref(&self) -> &[MnemonicIndex] {
        &self.0[..]
    }
}

impl Mnemonics {
    /// get the [`Type`](./enum.Type.html) of this given `Mnemonics`.
    ///
    /// # panic
    ///
    /// the only case this function may panic is if the `Mnemonics` has
    /// been badly constructed (i.e. not from one of the given smart
    /// constructor).
    ///
    pub fn get_type(&self) -> Type {
        Type::from_word_count(self.0.len()).unwrap()
    }

    /// get the mnemonic string representation in the given
    /// [`Language`](./dictionary/trait.Language.html).
    ///
    pub fn to_string<D>(&self, dic: &D) -> MnemonicString
    where
        D: dictionary::Language,
    {
        let mut vec = String::new();
        let mut first = true;
        for m in self.0.iter() {
            if first {
                first = false;
            } else {
                vec.push_str(dic.separator());
            }
            vec.push_str(&m.to_word(dic))
        }
        MnemonicString(vec)
    }

    /// Construct the `Mnemonics` from its string representation in the given
    /// [`Language`](./dictionary/trait.Language.html).
    ///
    /// # Error
    ///
    /// May fail with a [`LanguageError`](enum.Error.html#variant.LanguageError)
    /// if the given [`Language`](./dictionary/trait.Language.html) returns the
    /// given word is not within its dictionary.
    ///
    pub fn from_string<D>(dic: &D, mnemonics: &str) -> Result<Self>
    where
        D: dictionary::Language,
    {
        let mut vec = vec![];
        for word in mnemonics.split(dic.separator()) {
            vec.push(MnemonicIndex::from_word(dic, word)?);
        }
        Mnemonics::from_mnemonics(vec)
    }

    /// Construct the `Mnemonics` from the given array of `MnemonicIndex`.
    ///
    /// # Error
    ///
    /// May fail if this is an invalid number of `MnemonicIndex`.
    ///
    pub fn from_mnemonics(mnemonics: Vec<MnemonicIndex>) -> Result<Self> {
        let _ = Type::from_word_count(mnemonics.len())?;
        Ok(Mnemonics(mnemonics))
    }
}

pub mod dictionary {
    //! Language support for BIP39 implementations.
    //!
    //! We provide default dictionaries  for the some common languages.
    //! This interface is exposed to allow users to implement custom
    //! dictionaries.
    //!
    //! Because this module is part of the `cardano` crate and that we
    //! need to keep the dependencies as small as possible we do not support
    //! UTF8 NFKD by default. Users must be sure to compose (or decompose)
    //! our output (or input) UTF8 strings.
    //!

    use std::{error, fmt, result};

    use super::MnemonicIndex;

    /// Errors associated to a given language/dictionary
    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
    #[cfg_attr(feature = "generic-serialization", derive(Serialize, Deserialize))]
    pub enum Error {
        /// this means the given word is not in the Dictionary of the Language.
        MnemonicWordNotFoundInDictionary(String),
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
    impl error::Error for Error {}

    /// wrapper for `dictionary` operations that may return an error
    pub type Result<T> = result::Result<T, Error>;

    /// trait to represent the the properties that needs to be associated to
    /// a given language and its dictionary of known mnemonic words.
    ///
    pub trait Language {
        fn name(&self) -> &'static str;
        fn separator(&self) -> &'static str;
        fn lookup_mnemonic(&self, word: &str) -> Result<MnemonicIndex>;
        fn lookup_word(&self, mnemonic: MnemonicIndex) -> Result<String>;
    }

    /// Default Dictionary basic support for the different main languages.
    /// This dictionary expect the inputs to have been normalized (UTF-8 NFKD).
    ///
    /// If you wish to implement support for non pre-normalized form you can
    /// create reuse this dictionary in a custom struct and implement support
    /// for [`Language`](./trait.Language.html) accordingly (_hint_: use
    /// [`unicode-normalization`](https://crates.io/crates/unicode-normalization)).
    ///
    pub struct DefaultDictionary {
        pub words: [&'static str; 2048],
        pub name: &'static str,
    }
    impl Language for DefaultDictionary {
        fn name(&self) -> &'static str {
            self.name
        }
        fn separator(&self) -> &'static str {
            " "
        }
        fn lookup_mnemonic(&self, word: &str) -> Result<MnemonicIndex> {
            match self.words.iter().position(|x| x == &word) {
                None => Err(Error::MnemonicWordNotFoundInDictionary(word.to_string())),
                Some(v) => {
                    Ok(
                        // it is safe to call unwrap as we guarantee that the
                        // returned index `v` won't be out of bound for a
                        // `MnemonicIndex` (DefaultDictionary.words is an array of 2048 elements)
                        MnemonicIndex::new(v as u16).unwrap(),
                    )
                }
            }
        }
        fn lookup_word(&self, mnemonic: MnemonicIndex) -> Result<String> {
            Ok(unsafe { self.words.get_unchecked(mnemonic.0 as usize) }).map(|s| String::from(*s))
        }
    }

    /// default English dictionary as provided by the
    /// [BIP39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md#wordlists)
    ///
    pub const ENGLISH: DefaultDictionary = DefaultDictionary {
        words: include!("bip39_english.txt"),
        name: "english",
    };

    /// default French dictionary as provided by the
    /// [BIP39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md#french)
    ///
    pub const FRENCH: DefaultDictionary = DefaultDictionary {
        words: include!("bip39_french.txt"),
        name: "french",
    };

    /// default Japanese dictionary as provided by the
    /// [BIP39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md#japanese)
    ///
    pub const JAPANESE: DefaultDictionary = DefaultDictionary {
        words: include!("bip39_japanese.txt"),
        name: "japanese",
    };

    /// default Korean dictionary as provided by the
    /// [BIP39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md#japanese)
    ///
    pub const KOREAN: DefaultDictionary = DefaultDictionary {
        words: include!("bip39_korean.txt"),
        name: "korean",
    };

    /// default chinese simplified dictionary as provided by the
    /// [BIP39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md#chinese)
    ///
    pub const CHINESE_SIMPLIFIED: DefaultDictionary = DefaultDictionary {
        words: include!("bip39_chinese_simplified.txt"),
        name: "chinese-simplified",
    };
    /// default chinese traditional dictionary as provided by the
    /// [BIP39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md#chinese)
    ///
    pub const CHINESE_TRADITIONAL: DefaultDictionary = DefaultDictionary {
        words: include!("bip39_chinese_traditional.txt"),
        name: "chinese-traditional",
    };

    /// default italian dictionary as provided by the
    /// [BIP39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md#italian)
    ///
    pub const ITALIAN: DefaultDictionary = DefaultDictionary {
        words: include!("bip39_italian.txt"),
        name: "italian",
    };

    /// default spanish dictionary as provided by the
    /// [BIP39 standard](https://github.com/bitcoin/bips/blob/master/bip-0039/bip-0039-wordlists.md#spanish)
    ///
    pub const SPANISH: DefaultDictionary = DefaultDictionary {
        words: include!("bip39_spanish.txt"),
        name: "spanish",
    };
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::random;

    extern crate unicode_normalization;
    use self::unicode_normalization::UnicodeNormalization;

    use bip::bip39::dictionary::Language;

    #[test]
    fn english_dic() {
        let dic = &dictionary::ENGLISH;

        assert_eq!(dic.lookup_mnemonic("abandon"), Ok(MnemonicIndex(0)));
        assert_eq!(dic.lookup_mnemonic("crack"), Ok(MnemonicIndex(398)));
        assert_eq!(dic.lookup_mnemonic("shell"), Ok(MnemonicIndex(1579)));
        assert_eq!(dic.lookup_mnemonic("zoo"), Ok(MnemonicIndex(2047)));

        assert_eq!(dic.lookup_word(MnemonicIndex(0)), Ok("abandon".to_string()));
        assert_eq!(dic.lookup_word(MnemonicIndex(398)), Ok("crack".to_string()));
        assert_eq!(
            dic.lookup_word(MnemonicIndex(1579)),
            Ok("shell".to_string())
        );
        assert_eq!(dic.lookup_word(MnemonicIndex(2047)), Ok("zoo".to_string()));
    }

    #[test]
    fn mnemonic_zero() {
        let entropy = Entropy::Entropy12([0; 16]);
        let mnemonics = entropy.to_mnemonics();
        let entropy2 = Entropy::from_mnemonics(&mnemonics).unwrap();
        assert_eq!(entropy, entropy2);
    }

    #[test]
    fn mnemonic_7f() {
        let entropy = Entropy::Entropy12([0x7f; 16]);
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

    #[derive(Debug)]
    struct TestVector {
        entropy: &'static str,
        mnemonics: &'static str,
        seed: &'static str,
        passphrase: &'static str,
    }

    fn mk_test<D: dictionary::Language>(test: &TestVector, dic: &D) {
        // decompose the UTF8 inputs before processing:
        let mnemonics: String = test.mnemonics.nfkd().collect();
        let passphrase: String = test.passphrase.nfkd().collect();

        let mnemonics_ref = Mnemonics::from_string(dic, &mnemonics).expect("valid mnemonics");
        let mnemonics_str = MnemonicString::new(dic, mnemonics).expect("valid mnemonics string");
        let entropy_ref = Entropy::from_slice(&hex::decode(test.entropy).unwrap())
            .expect("decode entropy from hex");
        let seed_ref =
            Seed::from_slice(&hex::decode(test.seed).unwrap()).expect("decode seed from hex");

        assert!(mnemonics_ref.get_type() == entropy_ref.get_type());

        assert!(entropy_ref.to_mnemonics() == mnemonics_ref);
        assert!(
            entropy_ref
                == Entropy::from_mnemonics(&mnemonics_ref)
                    .expect("retrieve entropy from mnemonics")
        );

        assert_eq!(
            seed_ref,
            Seed::from_mnemonic_string(&mnemonics_str, passphrase.as_bytes())
        );
    }

    fn mk_tests<D: dictionary::Language>(tests: &[TestVector], dic: &D) {
        for test in tests {
            mk_test(test, dic);
        }
    }

    #[test]
    fn test_vectors_english() {
        mk_tests(TEST_VECTORS_ENGLISH, &dictionary::ENGLISH)
    }
    #[test]
    fn test_vectors_japanese() {
        mk_tests(TEST_VECTORS_JAPANESE, &dictionary::JAPANESE)
    }

    const TEST_VECTORS_ENGLISH: &'static [TestVector] = &include!("test_vectors/bip39_english.txt");
    const TEST_VECTORS_JAPANESE: &'static [TestVector] =
        &include!("test_vectors/bip39_japanese.txt");
}
