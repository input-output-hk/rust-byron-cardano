use std::{error, fmt, io, ops::Deref};

use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use tokio_codec as codec;

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct LightWeightConnectionId(u32);
impl Deref for LightWeightConnectionId {
    type Target = u32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl LightWeightConnectionId {
    const FIRST_NON_RESERVED_LIGHTWEIGHT_CONNECTION_ID: u32 = 1024;

    /// This is the first non reserved light weight connection identifier
    ///
    /// The value is `1024`.
    pub fn first_non_reserved() -> Self {
        LightWeightConnectionId(Self::FIRST_NON_RESERVED_LIGHTWEIGHT_CONNECTION_ID)
    }

    pub fn next(&mut self) -> Self {
        let current = *self;
        self.0 += 1;
        current
    }
}

impl fmt::Display for LightWeightConnectionId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

///
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum ControlHeader {
    CreateNewConnection = 0,
    CloseConnection = 1,
    CloseSocket = 2,
    CloseEndPoint = 3,
    ProbeSocket = 4,
    ProbeSocketAck = 5,
}
impl From<ControlHeader> for u32 {
    fn from(ch: ControlHeader) -> Self {
        ch as u32
    }
}

/// represent control commands or data exchanged between connections
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// open create or probe commands associated
    Control(ControlHeader, LightWeightConnectionId),

    /// send `Bytes` to the given `LightWeightConnectionId`.
    Data(LightWeightConnectionId, Bytes),
}
impl Event {
    pub fn expect_control(self) -> Result<(ControlHeader, LightWeightConnectionId), Self> {
        match self {
            Event::Control(ch, lwcid) => Ok((ch, lwcid)),
            event @ Event::Data(_, _) => Err(event),
        }
    }

    pub fn expect_data(self) -> Result<(LightWeightConnectionId, Bytes), Self> {
        match self {
            event @ Event::Control(_, _) => Err(event),
            Event::Data(lwcid, data) => Ok((lwcid, data)),
        }
    }
}

/// Decode Error that may happen while decoding the Event
#[derive(Debug)]
pub enum DecodeEventError {
    /// `tokio`'s I/O `Error`
    ///
    /// this is a requirement from `tokio`'s codec's `Decoder` trait
    /// that `DecodeEventError` implements `From<tokio::io::Error>`.
    IoError(io::Error),

    /// This means we have received an unknown `ControlHeader` or an
    /// *invalid* `ControlHeader`.
    ///
    /// includes value in range`[6..1024[`
    InvalidControlHeader(u32),

    /// The value is not a valid value for a `LightWeightConnectionId`
    ///
    /// includes value in range `[0..1024[`
    InvalidLightWeightConnectionId(u32),
}

impl From<io::Error> for DecodeEventError {
    fn from(e: io::Error) -> Self {
        DecodeEventError::IoError(e)
    }
}

impl fmt::Display for DecodeEventError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DecodeEventError::IoError(_) => write!(f, "I/O error"),
            DecodeEventError::InvalidControlHeader(n) => write!(f, "invalid control header {}", n),
            DecodeEventError::InvalidLightWeightConnectionId(n) => {
                write!(f, "invalid lightweight connection id {}", n)
            }
        }
    }
}

impl error::Error for DecodeEventError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DecodeEventError::IoError(e) => Some(e),
            DecodeEventError::InvalidControlHeader(_) => None,
            DecodeEventError::InvalidLightWeightConnectionId(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct EventCodec;
impl codec::Decoder for EventCodec {
    type Item = Event;
    type Error = DecodeEventError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // parse a given u32 and make sense of what is being read
        //
        enum ControlHeaderOrLightWeightConnectionId {
            // 0,1,2,3,4,5
            ControlHeader(ControlHeader),
            // [6..1024[
            UnknownControlHeader(u32),
            // [1024..]
            LightWeightConnectionId(LightWeightConnectionId),
        }
        impl From<u32> for ControlHeaderOrLightWeightConnectionId {
            fn from(v: u32) -> Self {
                match v {
                    0 => ControlHeaderOrLightWeightConnectionId::ControlHeader(
                        ControlHeader::CreateNewConnection,
                    ),
                    1 => ControlHeaderOrLightWeightConnectionId::ControlHeader(
                        ControlHeader::CloseConnection,
                    ),
                    2 => ControlHeaderOrLightWeightConnectionId::ControlHeader(
                        ControlHeader::CloseSocket,
                    ),
                    3 => ControlHeaderOrLightWeightConnectionId::ControlHeader(
                        ControlHeader::CloseEndPoint,
                    ),
                    4 => ControlHeaderOrLightWeightConnectionId::ControlHeader(
                        ControlHeader::ProbeSocket,
                    ),
                    5 => ControlHeaderOrLightWeightConnectionId::ControlHeader(
                        ControlHeader::ProbeSocketAck,
                    ),
                    6..=1023 => ControlHeaderOrLightWeightConnectionId::UnknownControlHeader(v),
                    v => ControlHeaderOrLightWeightConnectionId::LightWeightConnectionId(
                        LightWeightConnectionId(v),
                    ),
                }
            }
        }

        // we know we need at least 8 bytes
        if src.len() < 8 {
            return Ok(None);
        }

        // the bytes are not consumed yet
        // this is because we might have an incomplete frame or event
        // better keeping the complete transaction byte stream for now
        // an wait for more later.
        let (r, l) = {
            // we do the work in this scope for compatibility with rust edition 2015
            // when using `src[0..8]` we borrow an immutable reference to the `src`
            // but later we need to call `src.advance` which needs a mutable reference
            // and in 2015 the scope of the `header` is not dropped unless there is
            // an explicit scope for it.

            let mut header = src[0..8].into_buf();
            // either a ControlHeader or a LightWeightConnectionId
            let r = header.get_u32_be();
            // either a LightWeightConnectionId or a Length
            let l = header.get_u32_be();
            (r, l)
        };

        match r.into() {
            ControlHeaderOrLightWeightConnectionId::ControlHeader(ch) => {
                if l < LightWeightConnectionId::FIRST_NON_RESERVED_LIGHTWEIGHT_CONNECTION_ID {
                    Err(DecodeEventError::InvalidLightWeightConnectionId(l))
                } else {
                    // we can consume the 8 first bytes of the stream here
                    // it is r + l read earlier. We know the data is valid now
                    // so it is safe.
                    src.advance(8);
                    let lwcid = LightWeightConnectionId(l);
                    Ok(Some(Event::Control(ch, lwcid)))
                }
            }
            ControlHeaderOrLightWeightConnectionId::UnknownControlHeader(ch) => {
                Err(DecodeEventError::InvalidControlHeader(ch))
            }
            ControlHeaderOrLightWeightConnectionId::LightWeightConnectionId(lwcid) => {
                // the length of the data
                let len = l as usize;
                // the total length expected to be read from the stream
                let total_read = 8 + len;
                if src.len() < total_read {
                    // we are still missing bytes for the data, even though
                    // we have already read the first 8 bytes for the lwcid and
                    // the length we are not _advancing_ the stream just yet
                    // so when called again we are still in a valid state.
                    Ok(None)
                } else {
                    // here we have enough for the total length of the bytes to read
                    // we can advance the 8 first bytes already read (the r + l)
                    src.advance(8);
                    // consume the first `len` bytes from the stream
                    let bytes = src.split_to(len).freeze();
                    Ok(Some(Event::Data(lwcid, bytes)))
                }
            }
        }
    }
}
impl codec::Encoder for EventCodec {
    type Item = Event;
    type Error = io::Error;

    fn encode(&mut self, item: Event, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            Event::Control(ch, lwcid) => {
                dst.reserve(8);
                dst.put_u32_be(ch.into());
                dst.put_u32_be(*lwcid);
            }
            Event::Data(lwcid, bytes) => {
                dst.reserve(8 + bytes.len());
                dst.put_u32_be(*lwcid);
                dst.put_u32_be(bytes.len() as u32);
                dst.put(bytes);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
impl ::quickcheck::Arbitrary for LightWeightConnectionId {
    fn arbitrary<G: ::quickcheck::Gen>(g: &mut G) -> Self {
        let b = u32::arbitrary(g)
            | LightWeightConnectionId::FIRST_NON_RESERVED_LIGHTWEIGHT_CONNECTION_ID;
        LightWeightConnectionId(b)
    }
}
#[cfg(test)]
impl ::quickcheck::Arbitrary for ControlHeader {
    fn arbitrary<G: ::quickcheck::Gen>(g: &mut G) -> Self {
        let b = u32::arbitrary(g);
        match b % 6 {
            0 => ControlHeader::CreateNewConnection,
            1 => ControlHeader::CloseConnection,
            2 => ControlHeader::CloseSocket,
            3 => ControlHeader::CloseEndPoint,
            4 => ControlHeader::ProbeSocket,
            5 => ControlHeader::ProbeSocketAck,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
impl ::quickcheck::Arbitrary for Event {
    fn arbitrary<G: ::quickcheck::Gen>(g: &mut G) -> Self {
        let gen_control = <bool as ::quickcheck::Arbitrary>::arbitrary(g);
        let lwcid = LightWeightConnectionId::arbitrary(g);
        if gen_control {
            let ch = ControlHeader::arbitrary(g);
            Event::Control(ch, lwcid)
        } else {
            let bytes = <Vec<u8> as ::quickcheck::Arbitrary>::arbitrary(g);
            Event::Data(lwcid, bytes.into())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use tokio_codec::{Decoder, Encoder};

    quickcheck! {
        fn event_encode_decode(event: Event) -> bool {
            let mut codec = EventCodec;
            let mut stream = BytesMut::with_capacity(4_096);

            codec.encode(event.clone(), &mut stream).unwrap();

            let parsed = codec.decode(&mut stream).unwrap().unwrap();

            parsed == event
        }
    }
}
