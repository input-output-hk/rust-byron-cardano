use protocol;
use mstream::{MStream, MetricStart, MetricStats};
use wallet_crypto::config::{ProtocolMagic};
use rand;

pub struct Network(pub protocol::Connection<MStream>);

impl Network {
    pub fn new(protocol_magic: ProtocolMagic, host: &str) -> Self {
        let drg_seed = rand::random();
        let mut hs = protocol::packet::Handshake::default();
        hs.protocol_magic = protocol_magic;

        let stream = MStream::init(host);

        let conn = protocol::ntt::Connection::handshake(drg_seed, stream).unwrap();
        let mut conne = protocol::Connection::new(conn);
        conne.handshake(&hs).unwrap();
        Network(conne)
    }

    pub fn read_start(&self) -> MetricStart {
        MetricStart::new(self.0.get_backend().get_read_sz())
    }

    pub fn read_elapsed(&self, start: &MetricStart) -> MetricStats {
        start.diff(self.0.get_backend().get_read_sz())
    }
}

