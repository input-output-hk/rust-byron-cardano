pub mod hex {
    use super::{base_decode, base_encode};

    const ALPHABET : &'static str = "0123456789abcdef";

    pub fn encode(input: &[u8]) -> String {
        String::from_utf8(base_encode(ALPHABET, input)).unwrap()
    }
    pub fn decode(input: &str) -> Vec<u8> {
        base_decode(ALPHABET, input.as_bytes())
    }

    #[cfg(test)]
    mod tests {
        fn encode(input: &[u8], expected: &str) {
            let encoded = super::encode(input);
            assert_eq!(encoded, expected);
        }
        fn decode(expected: &[u8], input: &str) {
            let decoded = super::decode(input);
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
    use super::{base_decode, base_encode};

    const ALPHABET : &'static str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    pub fn encode(input: &[u8]) -> String {
        String::from_utf8(base_encode(ALPHABET, input)).unwrap()
    }
    pub fn decode(input: &str) -> Vec<u8> {
        base_decode(ALPHABET, input.as_bytes())
    }

    #[cfg(test)]
    mod tests {
        fn encode(input: &[u8], expected: &str) {
            let encoded = super::encode(input);
            assert_eq!(encoded, expected);
        }
        fn decode(expected: &[u8], input: &str) {
            let decoded = super::decode(input);
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
    }
}

pub fn base_encode(alphabet_s: &str, input: &[u8]) -> Vec<u8> {
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
    //let mut k = 0;
    //while (alphabet[k] ==

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


pub fn base_decode(alphabet_s: &str, input: &[u8]) -> Vec<u8> {
    let alphabet = alphabet_s.as_bytes();
    let base = alphabet.len() as u32;

    let mut bytes : Vec<u8> = vec![0];

    for i in 0..input.len() {
        let value = match alphabet.iter().position(|&x| x == input[i]) {
                    Some(idx) => idx,
                    None      => panic!()
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
    bytes.reverse();
    bytes
}
