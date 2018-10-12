pub mod protocol;

use std::io::{Write, Read};
use std::{iter, io, result, fmt, error};
use cardano::util::hex;

pub use self::protocol::{LightweightConnectionId, LIGHT_ID_MIN};

#[derive(Debug)]
pub enum Error {
    IOError(io::Error),
    UnsupportedVersion,
    InvalidLightid,
    InvalidRequest,
    CrossedRequest,
    UnknownErrorCode(u32),
    CommandFailed // TODO add command error in this sum type
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self { Error::IOError(e) }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IOError(_) => write!(f, "I/O error"),
            Error::UnsupportedVersion => write!(f, "Unsupported protocol version"),
            Error::InvalidRequest     => write!(f, "Invalid request"),
            Error::InvalidLightid     => write!(f, "Invalid lightid"),
            Error::CrossedRequest     => write!(f, "Crossed request"),
            Error::UnknownErrorCode(c) => write!(f, "Failed with an unknown error code: {}", c),
            Error::CommandFailed      => write!(f, "Command failed")
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<& error::Error> {
        match self {
            Error::IOError(ref error) => Some(error),
            _                         => None,
        }
    }
}

type Result<T> = result::Result<T, Error>;

pub struct Connection<W: Sized> {
    stream: W,
    drg: u64,
}

impl<W: Sized+Write+Read> Connection<W> {

    pub fn get_backend(&self) -> &W {
        &self.stream
    }

    pub fn handshake(drg_seed: u64, stream: W) -> Result<Self> {
        trace!("sending initial handshake");
        let mut conn = Connection { stream: stream, drg: drg_seed };
        let mut buf = vec![];
        protocol::handshake(&mut buf);
        conn.emit("handshake", &buf)?;
        match conn.recv_u32()? {
            0xffffffff => Err(Error::UnsupportedVersion),
            0x00000001 => Err(Error::InvalidRequest),
            0x00000002 => Err(Error::CrossedRequest),
            0x00000000 => { info!("HANDSHAKE OK"); Ok(conn) },
            v          => Err(Error::UnknownErrorCode(v)),
        }
    }

    pub fn get_nonce(&mut self) -> protocol::Nonce {
        let v = self.drg;
        self.drg += 1;
        v
    }

    pub fn create_light(&mut self, cid: LightweightConnectionId) -> Result<()> {
        let mut buf = vec![];
        protocol::create_conn(cid, &mut buf);
        self.emit("create-connection", &buf)
    }

    pub fn close_light(&mut self, cid: LightweightConnectionId) -> Result<()> {
        let mut buf = vec![];
        protocol::delete_conn(cid, &mut buf);
        self.emit("close-connection", &buf)
    }

    pub fn light_send_data(&mut self, lwc: LightweightConnectionId, dat: &[u8]) -> Result<()> {
        let mut buf = vec![];
        protocol::append_lightweight_data(lwc, dat.len() as u32, &mut buf);
        self.emit("send lightcon data header", &buf)?;
        self.emit("send lightcon data",  &dat)
    }

    // emit utility
    fn emit(&mut self, step: &str, dat: &[u8]) -> Result<()> {
        trace!("{}, bytes({}): {:?}", step, dat.len(), hex::encode(dat));
        self.stream.write_all(dat)?;
        Ok(())
    }

    // TODO some kind of error
    fn recv_u32(&mut self) -> Result<u32> {
        let mut buf = [0u8; 4];
        self.stream.read_exact(&mut buf)?;
        let v = ((buf[0] as u32) << 24) |
                ((buf[1] as u32) << 16) |
                ((buf[2] as u32) << 8) |
                (buf[3] as u32);
        Ok(v)
    }

    fn recv_cid(&mut self) -> Result<LightweightConnectionId> {
        let id = self.recv_u32()?;
        if id >= LIGHT_ID_MIN {
            Ok(LightweightConnectionId::new(id))
        } else {
            Err(Error::InvalidLightid)
        }
    }

    pub fn recv(&mut self) -> Result<protocol::Command>  {
        let hdr = self.recv_u32()?;
        if hdr < LIGHT_ID_MIN {
            match protocol::ControlHeader::from_u32(hdr) {
                Some(c)  => {
                    let r = self.recv_cid()?;
                    Ok(protocol::Command::Control(c, r))
                },
                None => Err(Error::CommandFailed)
            }
        } else {
            let len = self.recv_u32()?;
            Ok(protocol::Command::Data(LightweightConnectionId::new(hdr), len))
        }
    }

    pub fn recv_cmd(&mut self) -> Result<()> {
        let lwc = self.recv_u32()?;
        assert!(lwc < LIGHT_ID_MIN);
        let len = self.recv_u32()?;
        trace!("received lwc {} and len {}", lwc, len);
        Ok(())
    }

    pub fn recv_data(&mut self) -> Result<(LightweightConnectionId, Vec<u8>)> {
        let lwc = self.recv_u32()?;
        assert!(lwc >= LIGHT_ID_MIN);
        trace!("received data: {}", lwc);
        let len = self.recv_u32()?;
        let mut buf : Vec<u8> = iter::repeat(0).take(len as usize).collect();
        self.stream.read_exact(&mut buf[..])?;
        Ok((LightweightConnectionId::new(lwc),buf))
    }

    pub fn recv_len(&mut self, len: u32) -> Result<Vec<u8>> {
        let mut buf : Vec<u8> = iter::repeat(0).take(len as usize).collect();
        self.stream.read_exact(&mut buf[..])?;
        trace!("received({}): {:?}", buf.len(), hex::encode(&buf));
        Ok(buf)
    }
}
