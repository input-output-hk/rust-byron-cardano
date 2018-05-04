pub mod hex {
    //! simple implementation of hexadecimal encoding and decoding
    //!
    //! # Example
    //!
    //! ```
    //! use wallet_crypto::util::hex::{Error, encode, decode};
    //!
    //! let example = b"some bytes";
    //!
    //! assert!(example.as_ref() == decode(&encode(example)).unwrap().as_slice());
    //! ```
    //!
    use std::{result, fmt};

    const ALPHABET : &'static [u8] = b"0123456789abcdef";

    /// hexadecimal encoding/decoding potential errors
    #[derive(Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
    pub enum Error {
        /// error when a given character is not part of the supported
        /// hexadecimal alphabet. Contains the index of the faulty byte
        UnknownSymbol(usize),
    }
    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                &Error::UnknownSymbol(idx) => {
                    write!(f, "Unknown symbol at byte index {}", idx)
                }
            }
        }
    }

    pub type Result<T> = result::Result<T, Error>;

    /// encode bytes into an hexadecimal string
    ///
    ///  # Example
    ///
    /// ```
    /// use wallet_crypto::util::hex::{Error, encode};
    ///
    /// let example = b"some bytes";
    ///
    /// assert_eq!("736f6d65206279746573", encode(example));
    /// ```
    pub fn encode(input: &[u8]) -> String {
        let mut v = Vec::with_capacity(input.len() * 2);
        for &byte in input.iter() {
            v.push(ALPHABET[(byte >> 4) as usize]);
            v.push(ALPHABET[(byte & 0xf) as usize]);
        }

        unsafe {
            String::from_utf8_unchecked(v)
        }
    }

    /// decode the given hexadecimal string
    ///
    ///  # Example
    ///
    /// ```
    /// use wallet_crypto::util::hex::{Error, decode};
    ///
    /// let example = r"736f6d65206279746573";
    ///
    /// assert!(decode(example).is_ok());
    /// ```
    pub fn decode(input: &str) -> Result<Vec<u8>> {
        let mut b = Vec::with_capacity(input.len() / 2);
        let mut modulus = 0;
        let mut buf = 0;

        for (idx, byte) in input.bytes().enumerate() {
            buf <<= 4;

            match byte {
                b'A'...b'F' => buf |= byte - b'A' + 10,
                b'a'...b'f' => buf |= byte - b'a' + 10,
                b'0'...b'9' => buf |= byte - b'0',
                b' '|b'\r'|b'\n'|b'\t' => {
                    buf >>= 4;
                    continue
                }
                _ => {
                    return Err(Error::UnknownSymbol(idx));
                }
            }

            modulus += 1;
            if modulus == 2 {
                modulus = 0;
                b.push(buf);
            }
        }

        Ok(b)
    }

    #[cfg(test)]
    mod tests {
        fn encode(input: &[u8], expected: &str) {
            let encoded = super::encode(input);
            assert_eq!(encoded, expected);
        }
        fn decode(expected: &[u8], input: &str) {
            let decoded = super::decode(input).unwrap();
            assert_eq!(decoded.as_slice(), expected);
        }

        #[test]
        fn test_vector_1() {
            encode(&[1,2,3,4], "01020304");
            decode(&[1,2,3,4], "01020304");
        }

        #[test]
        fn test_vector_2() {
            encode(&[0xff,0x0f,0xff,0xff], "ff0fffff");
            decode(&[0xff,0x0f,0xff,0xff], "ff0fffff");
        }
    }
}

pub mod base58 {
    //! bitcoin's base58 encoding format
    //!
    //! # Example
    //!
    //! ```
    //! use wallet_crypto::util::base58;
    //!
    //! let encoded = r"TcgsE5dzphUWfjcb9i5";
    //! let decoded = b"Hello World...";
    //!
    //! assert_eq!(decoded, base58::decode(encoded).unwrap().as_slice());
    //! assert_eq!(encoded, base58::encode(decoded));
    //! ```

    pub const ALPHABET : &'static str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Serialize, Deserialize)]
    pub enum Error {
        /// error when a given character is not part of the supported
        /// base58 `ALPHABET`. Contains the index of the faulty byte.
        UnknownSymbol(usize)
    }
    impl ::std::fmt::Display for Error {
        fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            match self {
                &Error::UnknownSymbol(idx) => {
                    write!(f, "Unknown symbol at byte index {}", idx)
                }
            }
        }
    }

    pub type Result<T> = ::std::result::Result<T, Error>;

    /// encode in base58 the given input
    ///
    /// # Example
    ///
    /// ```
    /// use wallet_crypto::util::base58;
    ///
    /// let encoded = r"TcgsE5dzphUWfjcb9i5";
    /// let decoded = b"Hello World...";
    ///
    /// assert_eq!(encoded, base58::encode(decoded));
    /// ```
    pub fn encode(input: &[u8]) -> String {
        String::from_utf8(base_encode(ALPHABET, input)).unwrap()
    }

    /// decode from base58 the given input
    ///
    /// # Example
    ///
    /// ```
    /// use wallet_crypto::util::base58;
    ///
    /// let encoded = r"TcgsE5dzphUWfjcb9i5";
    /// let decoded = b"Hello World...";
    ///
    /// assert_eq!(decoded, base58::decode(encoded).unwrap().as_slice());
    /// ```
    pub fn decode(input: &str) -> Result<Vec<u8>> {
        base_decode(ALPHABET, input.as_bytes())
    }

    #[cfg(test)]
    mod tests {
        fn encode(input: &[u8], expected: &str) {
            let encoded = super::encode(input);
            assert_eq!(encoded, expected);
        }
        fn decode(expected: &[u8], input: &str) {
            let decoded = super::decode(input).unwrap();
            assert_eq!(decoded.as_slice(), expected);
        }

        #[test]
        fn test_vector_1() {
            encode(b"\0\0\0\0", "11111");
            decode(b"\0\0\0\0", "11111");
        }

        #[test]
        fn test_vector_2() {
            encode(b"This is awesome!", "BRY7dK2V98Sgi7CFWiZbap");
            decode(b"This is awesome!", "BRY7dK2V98Sgi7CFWiZbap");
        }

        #[test]
        fn test_vector_3() {
            encode(b"Hello World...", "TcgsE5dzphUWfjcb9i5");
            decode(b"Hello World...", "TcgsE5dzphUWfjcb9i5");
        }

        #[test]
        fn test_vector_4() {
            encode(b"\0abc", "1ZiCa");
            decode(b"\0abc", "1ZiCa");
        }

        #[test]
        fn test_vector_5() {
            encode(b"\0\0abc", "11ZiCa");
            decode(b"\0\0abc", "11ZiCa");
        }

        #[test]
        fn test_vector_6() {
            encode(b"\0\0\0abc", "111ZiCa");
            decode(b"\0\0\0abc", "111ZiCa");
        }

        #[test]
        fn test_vector_7() {
            encode(b"\0\0\0\0abc", "1111ZiCa");
            decode(b"\0\0\0\0abc", "1111ZiCa");
        }

        #[test]
        fn test_vector_8() {
            encode(b"abcdefghijklmnopqrstuvwxyz", "3yxU3u1igY8WkgtjK92fbJQCd4BZiiT1v25f");
            decode(b"abcdefghijklmnopqrstuvwxyz", "3yxU3u1igY8WkgtjK92fbJQCd4BZiiT1v25f");
        }
    }


    fn base_encode(alphabet_s: &str, input: &[u8]) -> Vec<u8> {
        let alphabet = alphabet_s.as_bytes();
        let base = alphabet.len() as u32;

        let mut digits = vec![0 as u8];
        for input in input.iter() {
            let mut carry = input.clone() as u32;
            for j in 0..digits.len() {
                carry = carry + ((digits[j] as u32) << 8);
                digits[j] = (carry % base) as u8;
                carry = carry / base;
            }

            while carry > 0 {
                digits.push((carry % base) as u8);
                carry = carry / base;
            }
        }

        let mut string = vec![];

        let mut k = 0;
        while (k < input.len()) && (input[k] == 0) {
            string.push(alphabet[0]);
            k += 1;
        }
        for digit in digits.iter().rev() {
            string.push(alphabet[digit.clone() as usize]);
        }

        string
    }


    fn base_decode(alphabet_s: &str, input: &[u8]) -> Result<Vec<u8>> {
        let alphabet = alphabet_s.as_bytes();
        let base = alphabet.len() as u32;

        let mut bytes : Vec<u8> = vec![0];
        let zcount = input.iter().take_while(|x| **x == alphabet[0]).count();

        for i in zcount..input.len() {
            let value = match alphabet.iter().position(|&x| x == input[i]) {
                        Some(idx) => idx,
                        None      => return Err(Error::UnknownSymbol(i))
                      };
            let mut carry = value as u32;
            for j in 0..bytes.len() {
                carry = carry + (bytes[j] as u32 * base);
                bytes[j] = carry as u8;
                carry = carry >> 8;
            }

            while carry > 0 {
                bytes.push(carry as u8);
                carry = carry >> 8;
            }
        }
        let leading_zeros = bytes.iter().rev().take_while(|x| **x == 0).count();
        if zcount > leading_zeros {
            if leading_zeros > 0 {
                for _ in 0..(zcount - leading_zeros - 1) { bytes.push(0); }
            } else {
                for _ in 0..zcount { bytes.push(0); }
            }
        }
        bytes.reverse();
        Ok(bytes)
    }
}