
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

#[cfg(test)]
mod tests {
    use super::{base_encode, base_decode};

    struct TestVector {
        msg: &'static [u8],
        res: &'static [u8]
    }

    const TEST_VECTORS : [TestVector;2] =
        [ TestVector {
            msg: b"This is awesome!",
            res: b"BRY7dK2V98Sgi7CFWiZbap"
          }
        , TestVector {
            msg: b"Hello World...",
            res: b"TcgsE5dzphUWfjcb9i5"
          }
        ];

    #[test]
    fn base58_encode() {
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

        for tv in TEST_VECTORS.iter() {
            let v = base_encode(&alphabet, tv.msg);
            assert_eq!(tv.res, v.as_slice());
        }
    }

    #[test]
    fn base58_decode() {
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

        for tv in TEST_VECTORS.iter() {
            let v = base_decode(&alphabet, tv.res);
            assert_eq!(tv.msg, v.as_slice());
        }
    }
}
