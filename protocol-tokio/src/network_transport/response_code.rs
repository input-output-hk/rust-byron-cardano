use std::{error, fmt};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ResponseCode {
    Success,
    InvalidRequest,
    CrossedRequest,
    UnsupportedVersion,
    UnknownErrorCode(u32),
}
impl From<ResponseCode> for u32 {
    fn from(rc: ResponseCode) -> u32 {
        match rc {
            ResponseCode::Success => 0x0000_0000,
            ResponseCode::InvalidRequest => 0x0000_0001,
            ResponseCode::CrossedRequest => 0x0000_0002,
            ResponseCode::UnsupportedVersion => 0xFFFF_FFFF,
            ResponseCode::UnknownErrorCode(v) => v,
        }
    }
}
impl From<u32> for ResponseCode {
    fn from(v: u32) -> Self {
        match v {
            0x00000000 => ResponseCode::Success,
            0x00000001 => ResponseCode::InvalidRequest,
            0x00000002 => ResponseCode::CrossedRequest,
            0xFFFFFFFF => ResponseCode::UnsupportedVersion,
            v => ResponseCode::UnknownErrorCode(v),
        }
    }
}
impl fmt::Display for ResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResponseCode::Success => write!(f, "Success"),
            ResponseCode::UnsupportedVersion => write!(f, "Unsupported version"),
            ResponseCode::InvalidRequest => write!(f, "Invalid request"),
            ResponseCode::CrossedRequest => write!(f, "Crossed request"),
            ResponseCode::UnknownErrorCode(code) => {
                write!(f, "Unknown error code {} (0x{:08X})", code, code)
            }
        }
    }
}
impl error::Error for ResponseCode {}

#[cfg(test)]
impl ::quickcheck::Arbitrary for ResponseCode {
    fn arbitrary<G: ::quickcheck::Gen>(g: &mut G) -> Self {
        ResponseCode::from(u32::arbitrary(g))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    quickcheck! {
        fn encode_decode(rc: ResponseCode) -> bool {
            let encoded : u32 = rc.into();
            let decoded : ResponseCode = encoded.into();
            rc == decoded
        }
    }
}
