
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
    #[test]
    fn base58() {
        let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        let expected = [1,2,3];

        let hex = [0x00,0x01,0x09,0x66,0x77,0x60,0x06,0x95,0x3D,0x55,0x67,0x43,0x9E,0x5E,0x39,0xF8,0x6A,0x0D,0x27,0x3B,0xEE,0xD6,0x19,0x67,0xF6];

        assert_eq!(base_decode(&alphabet, &base_encode(&alphabet, &expected)[..]), expected);

        //let r = base_encode(&alphabet, &hex);
        //assert_eq!(r, "16UwLL9Risc3QfPqBUvKofHmBQ7wMtjvM".as_bytes());

    }
}
