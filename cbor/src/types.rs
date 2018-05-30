
/// CBOR Major Types
///
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Type {
    UnsignedInteger,
    NegativeInteger,
    Bytes,
    Text,
    Array,
    Map,
    Tag,
    Special
}
impl Type {
    pub fn to_byte(self, len: u8) -> u8 {
        assert!(len <= 0b0001_1111);

        len | match self {
            Type::UnsignedInteger => 0b0000_0000,
            Type::NegativeInteger   => 0b0010_0000,
            Type::Bytes           => 0b0100_0000,
            Type::Text            => 0b0110_0000,
            Type::Array           => 0b1000_0000,
            Type::Map             => 0b1010_0000,
            Type::Tag             => 0b1100_0000,
            Type::Special         => 0b1110_0000
        }
    }
    pub fn from_byte(byte: u8) -> Type {
        match byte & 0b1110_0000 {
            0b0000_0000 => Type::UnsignedInteger,
            0b0010_0000 => Type::NegativeInteger,
            0b0100_0000 => Type::Bytes,
            0b0110_0000 => Type::Text,
            0b1000_0000 => Type::Array,
            0b1010_0000 => Type::Map,
            0b1100_0000 => Type::Tag,
            0b1110_0000 => Type::Special,
            _           => unreachable!()
        }
    }
}
impl From<u8> for Type {
    fn from(byte: u8) -> Type { Type::from_byte(byte) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_type_byte_encoding() {
        for i in 0b0000_0000..0b0001_1111 {
            assert!(Type::UnsignedInteger  == Type::from_byte(Type::to_byte(Type::UnsignedInteger, i)));
            assert!(Type::NegativeInteger  == Type::from_byte(Type::to_byte(Type::NegativeInteger, i)));
            assert!(Type::Bytes            == Type::from_byte(Type::to_byte(Type::Bytes,           i)));
            assert!(Type::Text             == Type::from_byte(Type::to_byte(Type::Text,            i)));
            assert!(Type::Array            == Type::from_byte(Type::to_byte(Type::Array,           i)));
            assert!(Type::Map              == Type::from_byte(Type::to_byte(Type::Map,             i)));
            assert!(Type::Tag              == Type::from_byte(Type::to_byte(Type::Tag,             i)));
            assert!(Type::Special          == Type::from_byte(Type::to_byte(Type::Special,         i)));
        }
    }
}
