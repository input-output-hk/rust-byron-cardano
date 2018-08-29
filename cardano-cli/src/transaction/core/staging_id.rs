use rand;
use storage::utils::serialize::{SIZE_SIZE, read_size, write_size};
use cardano::util::{base58};
use std::{str, fmt};
use serde::{ser, de};

/// a Staging ID represents a transaction under construction
///
/// This will be the way the user will refer to the transaction when
/// preparing it, reviewing it and signing it.
///
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct StagingId(u32);
impl StagingId {
    /// Generate a new random `StagingId`
    pub fn generate() -> Self {
        StagingId(rand::random())
    }

    fn to_base58(self) -> String {
        let mut buf = [0u8;SIZE_SIZE];
        write_size(&mut buf, self.0);
        base58::encode(&buf)
    }
    fn from_base58(buf: &str) -> Result<Self, ParseStagingIdError> {
        let buf = base58::decode(buf).map_err(ParseStagingIdError::Base58Encoding)?;
        if buf.len() != SIZE_SIZE {
            Err(ParseStagingIdError::InvalidSize(buf.len()))
        } else {
            Ok(StagingId(read_size(&buf)))
        }
    }
}
impl fmt::Display for StagingId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}
impl str::FromStr for StagingId {
    type Err = ParseStagingIdError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        StagingId::from_base58(s)
    }
}

struct StagingIdVisitor;
impl<'de> de::Visitor<'de> for StagingIdVisitor {
    type Value = StagingId;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a valid StagingId")
    }
    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
        where E: de::Error
    { Ok(StagingId(v as u32)) }
    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
        where E: de::Error
    { Ok(StagingId(v as u32)) }
    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
        where E: de::Error
    { Ok(StagingId(v)) }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where E: de::Error
    {
        match v.parse() {
            Ok(v) => Ok(v),
            Err(err) => Err(E::custom(err))
        }
    }
}
impl ser::Serialize for StagingId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: ser::Serializer
    {
        if serializer.is_human_readable() {
            serializer.serialize_str(&self.to_base58())
        } else {
            serializer.serialize_u32(self.0)
        }
    }
}
impl<'de> de::Deserialize<'de> for StagingId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: de::Deserializer<'de>
    {
        if deserializer.is_human_readable() {
            deserializer.deserialize_str(StagingIdVisitor)
        } else {
            deserializer.deserialize_u32(StagingIdVisitor)
        }
    }
}

#[derive(Debug)]
pub enum ParseStagingIdError {
    Base58Encoding(base58::Error),
    InvalidSize(usize)
}
impl fmt::Display for ParseStagingIdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseStagingIdError::Base58Encoding(err) => {
                write!(f, "not a valid base58 encoding: {}", err)
            },
            ParseStagingIdError::InvalidSize(found) => {
                write!(f, "not of a valid size (expected {}, but got {})", SIZE_SIZE, found)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn test_encode_decode(id: StagingId) {
        let encoded = id.to_string();
        let decoded = encoded.parse::<StagingId>().unwrap();

        assert!(id == decoded, "encode/decode failed for id: {:?} (encoded: {:?}, decoded: {:?})", id, encoded, decoded);
    }

    #[test]
    fn randoms() {
        for _ in 0..100 {
            test_encode_decode(StagingId::generate())
        }
    }

    #[test]
    fn goldens() {
        for (i, (id, encoded)) in GOLDEN_TESTS.iter().enumerate() {
            let encoded_id = id.to_string();
            let decoded_id = encoded.parse::<StagingId>().unwrap();

            assert!(&encoded_id == encoded, "encoding Id {:?} (index: {})", id, i);
            assert!(&decoded_id == id,      "decoding Id {:?} (index: {})", id, i);
        }
    }

    const GOLDEN_TESTS : [(StagingId, &'static str);100] =
        [ (StagingId(3234220616), "5voE7R")
        , (StagingId(1388822340), "37j5Vm")
        , (StagingId(2303964901), "4WbRVn")
        , (StagingId(762492905), "2ANyaQ")
        , (StagingId(3004347187), "5aV4kS")
        , (StagingId(2182279703), "4Kqkj8")
        , (StagingId(2506060569), "4pTDVr")
        , (StagingId(3277466363), "5zcsZk")
        , (StagingId(2754950021), "5CSqfJ")
        , (StagingId(2497428513), "4ogyVJ")
        , (StagingId(3160675640), "5pJHkb")
        , (StagingId(2783502164), "5EyBDd")
        , (StagingId(1060424882), "2chxNd")
        , (StagingId(1711174911), "3cDDgi")
        , (StagingId(1863354502), "3qfBNm")
        , (StagingId(1625664781), "3UexW8")
        , (StagingId(2437232400), "4iNTHH")
        , (StagingId(821928469), "2FdbhW")
        , (StagingId(997088803), "2X7LmG")
        , (StagingId(3000422352), "5a8x2s")
        , (StagingId(773511876), "2BMT8f")
        , (StagingId(1477850793), "3FbNZ6")
        , (StagingId(1261147553), "2vSiEL")
        , (StagingId(1718038245), "3cpPv4")
        , (StagingId(2897624137), "5R45hr")
        , (StagingId(3773428170), "6kSojP")
        , (StagingId(2701029304), "57gUum")
        , (StagingId(2963242512), "5WrPm9")
        , (StagingId(4208919179), "7Qvp34")
        , (StagingId(2950532477), "5VjFWc")
        , (StagingId(3580979302), "6TSTPT")
        , (StagingId(3687479239), "6crJ7G")
        , (StagingId(1103976498), "2gZAkZ")
        , (StagingId(726284668), "27BQ8T")
        , (StagingId(731425455), "27dkJi")
        , (StagingId(2825085414), "5JeJUD")
        , (StagingId(3567745901), "6SGdZN")
        , (StagingId(4289657918), "7Y4crH")
        , (StagingId(2970492733), "5XVYzx")
        , (StagingId(2372670421), "4cfZFi")
        , (StagingId(2761873001), "5D4Kd2")
        , (StagingId(446400594), "gSvLy")
        , (StagingId(1860049846), "3qNF1w")
        , (StagingId(2619708479), "4zVh4a")
        , (StagingId(2734813565), "5Afdo2")
        , (StagingId(1577097332), "3QN36K")
        , (StagingId(133616755), "CopcA")
        , (StagingId(2094333829), "4C51Ux")
        , (StagingId(67457382), "6xjjK")
        , (StagingId(2277560814), "4UG6Uu")
        , (StagingId(502810274), "mS2yf")
        , (StagingId(1733882140), "3eDbkf")
        , (StagingId(2887268023), "5Q91CA")
        , (StagingId(98464474), "9hf4M")
        , (StagingId(1022293838), "2ZLXLq")
        , (StagingId(3730422077), "6gePX2")
        , (StagingId(1075025381), "2dznbN")
        , (StagingId(2208471081), "4N9zWc")
        , (StagingId(1117771586), "2hmsYu")
        , (StagingId(3797007687), "6nXf6n")
        , (StagingId(3228822310), "5vKZPB")
        , (StagingId(1273354296), "2wXGsR")
        , (StagingId(2182391259), "4KrKtW")
        , (StagingId(83376786), "8NL1o")
        , (StagingId(692152161), "24ATix")
        , (StagingId(794995186), "2DFZNZ")
        , (StagingId(1770732144), "3hUTyZ")
        , (StagingId(620780966), "wrfZ7")
        , (StagingId(1516957719), "3K3ogS")
        , (StagingId(392670610), "bhYHw")
        , (StagingId(516192810), "ncd93")
        , (StagingId(360775982), "Yt58Z")
        , (StagingId(2203167627), "4Mgoyk")
        , (StagingId(186687759), "HVpmU")
        , (StagingId(2155971408), "4HWvBq")
        , (StagingId(4007928394), "77AgUh")
        , (StagingId(1976450441), "41eprk")
        , (StagingId(1231118923), "2snomL")
        , (StagingId(649105678), "zMqWD")
        , (StagingId(2504117312), "4pHFqR")
        , (StagingId(3882901167), "6v7tEr")
        , (StagingId(1151510746), "2kko2Z")
        , (StagingId(1583700852), "3Qwt67")
        , (StagingId(1950015880), "3yKLnT")
        , (StagingId(3737124192), "6hEjpj")
        , (StagingId(2297016362), "4VyowP")
        , (StagingId(3685560183), "6cgTe6")
        , (StagingId(1541148040), "3MBncw")
        , (StagingId(513619682), "nPSEm")
        , (StagingId(2021010402), "45bCyw")
        , (StagingId(1680982502), "3ZYUXw")
        , (StagingId(660938153), "21QUtQ")
        , (StagingId(1625820860), "3Ufku9")
        , (StagingId(258199845), "PpLpU")
        , (StagingId(3687068905), "6cpC8Y")
        , (StagingId(2183653103), "4KxnzS")
        , (StagingId(2270860298), "4Tfkem")
        , (StagingId(778716005), "2Bp892")
        , (StagingId(2688547790), "56aWbF")
        , (StagingId(3625433771), "6XNJ9p")
        ];
}
