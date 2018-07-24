use protocol;
use mstream::{MStream, MetricStart, MetricStats};
use cardano::{config::{ProtocolMagic}, util::{hex}};
use rand;
use std::{net::{SocketAddr, ToSocketAddrs}, ops::{Deref, DerefMut}};
use cardano::block::{self, Block, BlockHeader, RawBlock, HeaderHash, EpochId, SlotId, BlockDate};
use storage::{self, Storage, types::{PackHash}};
use protocol::command::*;
use std::time::{SystemTime, Duration};

use config::net;
use network::{Error, Result};
use network::api::{Api, FetchEpochParams, FetchEpochResult};

/// native peer
pub struct PeerPool {
    pub name: String,

    /// the domain or the IP address of the peer
    pub address: String,

    /// there is at least one connection
    ///
    /// multiple connections if the IP addresses are different
    /// when contacting the DNS resolver
    pub connections: Vec<Connection>
}
impl PeerPool {
    pub fn new(name: String, address: String, protocol_magic: ProtocolMagic) -> Result<Self> {
        let mut connections = Vec::new();
        for sockaddr in address.to_socket_addrs()? {
            match Connection::new(sockaddr, protocol_magic) {
                Ok(connection) => connections.push(connection),
                Err(Error::ConnectionTimedOut) => {
                    warn!("connection peer `{}' address {} timed out, ignoring for now.", name, sockaddr)
                },
                Err(err) => {
                    error!("connection peer `{}' address {} failed: {:?}", name, sockaddr, err);
                    return Err(err)
                },
            }
        }
        Ok(PeerPool { name, address, connections })
    }
}

// TODO: this is not necessarily what we want to do here,
//
// in the case we have multiple connection on a peer, we might want to operate
// paralellisation of the effort
impl Api for PeerPool {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_tip()
        }
    }

    fn get_block(&mut self, hash: HeaderHash) -> Result<RawBlock> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_block(hash)
        }
    }

    fn get_blocks(&mut self, from: HeaderHash, to: HeaderHash) -> Result<Vec<(HeaderHash, RawBlock)>> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.get_blocks(from, to)
        }
    }

    fn fetch_epoch(&mut self, config: &net::Config, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult> {
        match self.connections.get_mut(0) {
            None => panic!("We expect at lease one connection on any native peer"),
            Some(conn) => conn.fetch_epoch(config, storage, fep)
        }
    }
}

pub struct Connection(pub SocketAddr, pub OpenPeer);
impl Connection {
    pub fn new(sockaddr: SocketAddr, protocol_magic: ProtocolMagic) -> Result<Self> {
        let network = OpenPeer::new(protocol_magic, &sockaddr)?;
        Ok(Connection (sockaddr, network))
    }
}
impl Deref for Connection {
    type Target = OpenPeer;
    fn deref(&self) -> &Self::Target { &self.1 }
}
impl DerefMut for Connection {
    fn deref_mut(&mut self) -> &mut Self::Target { & mut self.1 }
}

pub struct OpenPeer(pub protocol::Connection<MStream>);

impl OpenPeer {
    pub fn new(protocol_magic: ProtocolMagic, host: &SocketAddr) -> Result<Self> {
        let drg_seed = rand::random();
        let mut hs = protocol::packet::Handshake::default();
        hs.protocol_magic = protocol_magic;

        let stream = MStream::init(host)?;

        let conn = protocol::ntt::Connection::handshake(drg_seed, stream)?;
        let mut conne = protocol::Connection::new(conn);
        conne.handshake(&hs)?;
        Ok(OpenPeer(conne))
    }

    pub fn read_start(&self) -> MetricStart {
        MetricStart::new(self.0.get_backend().get_read_sz())
    }

    pub fn read_elapsed(&self, start: &MetricStart) -> MetricStats {
        start.diff(self.0.get_backend().get_read_sz())
    }
}
impl Api for OpenPeer {
    fn get_tip(&mut self) -> Result<BlockHeader> {
        let block_headers_raw = GetBlockHeader::tip().execute(&mut self.0).expect("to get one header at least");

        let block_headers = block_headers_raw.decode()?;

        if block_headers.len() != 1 {
            panic!("get head header return more than 1 header")
        }
        Ok(block_headers[0].clone())
    }

    fn get_block(&mut self, hash: HeaderHash) -> Result<RawBlock> {
        let b = GetBlock::only(&hash).execute(&mut self.0)
            .expect("to get one block at least");

        Ok(RawBlock::from_dat(b[0].as_ref().to_vec()))
    }

    fn get_blocks(&mut self, from: HeaderHash, to: HeaderHash) -> Result<Vec<(HeaderHash, RawBlock)>> {
        let mut blocks = vec!();
        fetch_range(self, &from, &to, false, |blockhash, _block, block_raw| {
            blocks.push((blockhash.clone(), block_raw.clone()));
        });
        Ok(blocks)
    }

    fn fetch_epoch(&mut self, _config: &net::Config, storage: &mut Storage, fep: FetchEpochParams) -> Result<FetchEpochResult> {
        let result = download_epoch(storage, self, fep.epoch_id, &fep.start_header_hash, &fep.previous_header_hash, &fep.upper_bound_hash);
        Ok(FetchEpochResult {
            last_header_hash: result.0,
            next_epoch_hash: Some(result.1),
            packhash: result.2
        })
    }
}

/// Fetch the blocks in the half-open interval (from, to] in batches
/// of around 2000 blocks at a time. The closure 'got_block' is called
/// with each block in the order (from, to]. We check that there is no
/// gap in the chain, i.e. each block has the previous block as its
/// parent. If 'one_epoch' is set, we return when we encounter a block
/// in a different epoch than the previous one.
fn fetch_range<F>(
    net: &mut OpenPeer,
    from: &HeaderHash,
    to: &HeaderHash,
    one_epoch: bool,
    mut got_block: F)
    -> (HeaderHash, Option<HeaderHash>)
where F: FnMut(&HeaderHash, &Block, &RawBlock) -> ()
{
    assert!(from != to);

    let mut from = from.clone();
    let mut epoch = None;
    let mut next_hash = None;

    while &from != to && (!one_epoch || next_hash.is_none()) {
        info!("  ### from={} to={}", from, to);
        let metrics = net.read_start();
        let block_headers_raw = GetBlockHeader::range(
            &vec![from.clone()], to.clone()).execute(&mut net.0).expect("to get one header at least");
        let hdr_metrics = net.read_elapsed(&metrics);
        let block_headers = block_headers_raw.decode().unwrap();
        info!("  got {} headers  ( {} )", block_headers.len(), hdr_metrics);

        assert!(!block_headers.is_empty());

        let mut start = 0;
        let end = block_headers.len() - 1;

        info!("  asked {} to {}", from, to);
        info!("  start {} {} <- {}", block_headers[start].compute_hash(), block_headers[start].get_blockdate(), block_headers[start].get_previous_header());
        info!("  end   {} {} <- {}", block_headers[end].compute_hash(), block_headers[end].get_blockdate(), block_headers[end].get_previous_header());

        // The server will return the oldest ~2000 blocks starting at
        // 'from'. However, they're in reverse order. Thus the last
        // element of 'block_headers' should have 'from' as its
        // parent.
        assert!(block_headers[end].get_previous_header() == from);

        if one_epoch {

            if epoch.is_none() {
                epoch = Some(block_headers[end].get_blockdate().get_epochid());
            }

            // Skip blocks beyond the current epoch.
            while end >= start && block_headers[start].get_blockdate().get_epochid() > epoch.unwrap() {
                start += 1
            }

            if start > 0 {
                info!("  found next epoch");
                next_hash = Some(block_headers[start-1].compute_hash());
            }
        }

        let latest_block = &block_headers[start];
        let first_block = &block_headers[end];

        info!("  hdr latest {} {}", latest_block.compute_hash(), latest_block.get_blockdate());
        info!("  hdr first  {} {}", first_block.compute_hash(), first_block.get_blockdate());

        let download_start_hash = first_block.compute_hash();

        let metrics = net.read_start();
        let blocks_raw = GetBlock::from(&download_start_hash, &latest_block.compute_hash())
                                .execute(&mut net.0)
                                .expect("to get one block at least");
        let blocks_metrics = net.read_elapsed(&metrics);
        info!("  got {} blocks  ( {} )", blocks_raw.len(), blocks_metrics);

        assert!(!blocks_raw.is_empty());

        for block_raw in blocks_raw.iter() {
            let block = block_raw.decode().unwrap();
            let hdr = block.get_header();
            let date = hdr.get_blockdate();
            let blockhash = hdr.compute_hash();

            info!("got block {} {} prev {}", blockhash, date, hdr.get_previous_header());

            if hdr.get_previous_header() != from {
                panic!("previous header doesn't match: hash {} date {} got {} expected {}",
                       blockhash, date, hdr.get_previous_header(), from)
            }

            if one_epoch && date.get_epochid() != epoch.unwrap() {
                panic!("received block from wrong epoch: hash {} date {} expected {}",
                       blockhash, date, epoch.unwrap())
            }

            got_block(&hdr.compute_hash(), &block, &block_raw);

            from = blockhash;
        }
    }

    (from, next_hash)
}

fn download_epoch(storage: &Storage, net: &mut OpenPeer,
                  epoch_id: EpochId,
                  x_start_hash: &HeaderHash,
                  x_previous_headerhash: &HeaderHash,
                  tip_hash: &HeaderHash) -> (HeaderHash, HeaderHash, PackHash)
{
    let mut writer = storage::pack::PackWriter::init(&storage.config);
    let epoch_time_start = SystemTime::now();
    let mut expected_slotid = block::BlockDate::Genesis(epoch_id);

    let (last_hash, next_hash) = fetch_range(net, x_previous_headerhash, tip_hash, true, |blockhash, block, block_raw| {
        let hdr = block.get_header();
        let date = hdr.get_blockdate();

        if date.get_epochid() != epoch_id {
            panic!("trying to append a block of different epoch id {}", date.get_epochid())
        }

        if &date != &expected_slotid {
            println!("  WARNING: not contiguous. addr {} found, expected {}", date, expected_slotid);
        }

        match date {
            BlockDate::Genesis(epoch) => {
                expected_slotid = BlockDate::Normal(SlotId { epoch: epoch, slotid: 0 });
            },
            BlockDate::Normal(slotid) => {
                expected_slotid = BlockDate::Normal(slotid.next());
            },
        }

        writer.append(&storage::types::header_to_blockhash(&blockhash), block_raw.as_ref());
    });

    // write packfile
    let (packhash, index) = writer.finalize();
    let (_, tmpfile) = storage::pack::create_index(storage, &index);
    tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();
    let epoch_time_elapsed = epoch_time_start.elapsed().unwrap();
    info!("=> pack {} written for epoch {} in {}", hex::encode(&packhash[..]), epoch_id, duration_print(epoch_time_elapsed));
    (last_hash, next_hash.unwrap(), packhash)
}

/*
fn download_epoch(storage: &Storage, net: &mut OpenPeer,
                  epoch_id: EpochId,
                  x_start_hash: &HeaderHash,
                  x_previous_headerhash: &HeaderHash,
                  tip_hash: &HeaderHash) -> (HeaderHash, HeaderHash, PackHash) {
    let mut start_hash = x_start_hash.clone();
    let mut found_epoch_boundary = None;
    let mut previous_headerhash = x_previous_headerhash.clone();
    let mut expected_slotid = block::BlockDate::Genesis(epoch_id);

    loop {
        info!("  ### slotid={} from={}", expected_slotid, start_hash);
        let metrics = net.read_start();
        let block_headers_raw = network_get_blocks_headers(net, &start_hash, tip_hash);
        let hdr_metrics = net.read_elapsed(&metrics);
        let block_headers = block_headers_raw.decode().unwrap();
        info!("  got {} headers  ( {} )", block_headers.len(), hdr_metrics);

        let mut start = 0;
        let mut end = block_headers.len() - 1;

        debug!("  asked {} to {}", start_hash, tip_hash);
        debug!("  start {} {} <- {}", block_headers[start].compute_hash(),  block_headers[start].get_blockdate(), block_headers[start].get_previous_header());
        debug!("  end   {} {} <- {}", block_headers[end].compute_hash(), block_headers[end].get_blockdate(), block_headers[end].get_previous_header());

        // if the earliest block headers we receive has an epoch
        // less than the expected epoch, we just fast skip
        // this set of headers and restart the loop with the
        // latest known hash
        if block_headers[start].get_blockdate().get_epochid() < epoch_id {
            start_hash = block_headers[start].compute_hash();
            info!("headers are of previous epochs, fast skip to {}", start_hash);
            continue;
        }

        while end >= start && block_headers[start].get_blockdate().get_epochid() > epoch_id {
            start += 1
        }
        while end > start && block_headers[end].get_blockdate().get_epochid() < epoch_id {
            end -= 1
        }

        if start > 0 {
            info!("  found next epoch");
            found_epoch_boundary = Some(block_headers[start-1].compute_hash());
        }
        let latest_block = &block_headers[start];
        let first_block = &block_headers[end];

        debug!("  hdr latest {} {}", latest_block.compute_hash(), latest_block.get_blockdate());
        debug!("  hdr first  {} {}", first_block.compute_hash(), first_block.get_blockdate());

        let download_start_hash = if first_block.get_blockdate() == expected_slotid {
            first_block.compute_hash()
        } else if first_block.get_blockdate() == expected_slotid.next() {
            first_block.get_previous_header()
        } else {
            panic!("not matching. gap")
        };

        let metrics = net.read_start();
        let blocks_raw = GetBlock::from(&download_start_hash, &latest_block.compute_hash())
                                .execute(&mut net.0)
                                .expect("to get one block at least");
        let blocks_metrics = net.read_elapsed(&metrics);
        info!("  got {} blocks  ( {} )", blocks_raw.len(), blocks_metrics);

        let first_block = blocks_raw[0].decode().unwrap();
        let first_block_hdr = first_block.get_header();
        debug!("first block {} {} prev {}", first_block_hdr.compute_hash(), first_block_hdr.get_blockdate(), first_block_hdr.get_previous_header());

        for block_raw in blocks_raw.iter() {
            let block = block_raw.decode().unwrap();
            let hdr = block.get_header();
            let date = hdr.get_blockdate();
            let blockhash = hdr.compute_hash();
            let block_previous_header = hdr.get_previous_header();

            if date.get_epochid() != epoch_id {
                panic!("trying to append a block of different epoch id {}", date.get_epochid())
            }

            if previous_headerhash != block_previous_header {
                panic!("previous header doesn't match: hash {} date {} got {} expected {}",
                       blockhash, date, block_previous_header, previous_headerhash)
            }

            if &date != &expected_slotid {
                println!("  WARNING: not contiguous. addr {} found, expected {} {}", date, expected_slotid, block_previous_header);
            }

            match date {
                BlockDate::Genesis(epoch) => {
                    expected_slotid = BlockDate::Normal(SlotId { epoch: epoch, slotid: 0 });
                },
                BlockDate::Normal(slotid) => {
                    expected_slotid = BlockDate::Normal(slotid.next());
                },
            }

            writer.append(&storage::types::header_to_blockhash(&blockhash), block_raw.as_ref());
            previous_headerhash = blockhash.clone();
        }
        // println!("packing {}", slot);
        start_hash = previous_headerhash.clone();

        match found_epoch_boundary {
            None    => {},
            Some(b) => {
                info!("=> packing finished {} slotids", expected_slotid);
                // write packfile
                let (packhash, index) = writer.finalize();
                let (_, tmpfile) = storage::pack::create_index(storage, &index);
                tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();
                let epoch_time_elapsed = epoch_time_start.elapsed().unwrap();
                info!("=> pack {} written for epoch {} in {}", hex::encode(&packhash[..]), epoch_id, duration_print(epoch_time_elapsed));
                return (previous_headerhash, b, packhash)
            },
        }
    }
}
*/

fn duration_print(d: Duration) -> String {
    format!("{}.{:03} seconds", d.as_secs(), d.subsec_millis())
}
