mod connecting;
mod codec;

use super::network_transport as nt;

use tokio::prelude::{*};

pub use self::connecting::{Connecting};
pub use self::codec::{*};

pub struct Connection<T> {
    connection: nt::Connection<T>,

    next_lightweight_connection_id: nt::LightWeightConnectionId,

    next_node_id: NodeId,
}

impl<T: AsyncRead+AsyncWrite> Connection<T> {
    fn new(connection: nt::Connection<T>) -> Self {
        Connection {
            connection: connection,

            next_lightweight_connection_id: nt::LightWeightConnectionId::first_non_reserved(),
            next_node_id: NodeId::default(),

        }
    }

    pub fn connect(inner: T) -> Connecting<T> { Connecting::new(inner) }
}
