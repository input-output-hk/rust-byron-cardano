use tokio::prelude::{*};
use futures::{StartSend, Poll};

use super::{nt, Connection};

pub use self::nt::ConnectingError;

pub enum ConnectingState<T> {
    NtConnecting(nt::Connecting<T>),
    NtHandshake(Connection<T>),
    Consummed,
}

pub struct Connecting<T> {
    state: ConnectingState<T>
}

impl<T: AsyncRead+AsyncWrite> Connecting<T> {
    pub fn new(inner: T) -> Self {
        Connecting {
            state: ConnectingState::NtConnecting(nt::Connection::connect(inner)),
        }
    }
}

impl<T: AsyncRead+AsyncWrite> Future for Connecting<T> {
    type Item = Connection<T>;
    type Error = ConnectingError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let connection = if let ConnectingState::NtConnecting(mut nt) = self.state {
                let nt = try_ready!(nt.poll());
                Connection::new(nt)
            };
        }
    }
}
