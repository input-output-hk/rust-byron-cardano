use std::{error, fmt};

/// Represents errors that can be returned by the node client implementation.
#[derive(Debug)]
pub struct Error {
    kind: String,
    source: Box<dyn error::Error + Send + Sync>,
}

pub enum ErrorKind {
    Handshake,
    Connect,
}

impl Error {
    pub fn new<E>(kind: ErrorKind, source: E) -> Self
    where
        E: Into<Box<dyn error::Error + Send + Sync>>,
    {
        Error {
            kind,
            source: source.into(),
        }
    }

    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.kind {
            ErrorKind::Handshake=> write!(f, "error during handshake"),
            ErrorKind::Connect => write!(f, "error during connection"),
        }
    }
}
