mod result;
mod error;
mod types;
mod len;
pub mod de;
pub mod se;
mod macros;

pub use len::{*};
pub use types::{*};
pub use result::{Result};
pub use error::{Error};
pub use de::{Deserialize};
pub use se::{Serialize};

const MAX_INLINE_ENCODING : u64 = 23;

const CBOR_PAYLOAD_LENGTH_U8  : u8 = 24;
const CBOR_PAYLOAD_LENGTH_U16 : u8 = 25;
const CBOR_PAYLOAD_LENGTH_U32 : u8 = 26;
const CBOR_PAYLOAD_LENGTH_U64 : u8 = 27;

pub fn test_encode_decode<V: Sized+Eq+Serialize+Deserialize>(v: &V) -> Result<bool> {
    let bytes = Serialize::serialize(v, se::Serializer::new())?.finalize();

    let mut raw = de::RawCbor::from(&bytes);
    let v_ = Deserialize::deserialize(&mut raw)?;

    Ok(v == &v_)
}
