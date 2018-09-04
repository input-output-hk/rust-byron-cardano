
use exe_common;
use exe_common::network::{api::Api, api::BlockRef};
use cardano::{block::{BlockDate, EpochId, HeaderHash}, tx::{TxAux}};
use utils::term::Term;
use storage::{self, tag};
use std::ops::Deref;
use std::time::SystemTime;
use std::mem;

pub struct ConnectedPeer<'a> {
    peer: Peer<'a>,
    connection: exe_common::network::Peer
}
impl<'a> Deref for ConnectedPeer<'a> {
    type Target = Peer<'a>;
    fn deref(&self) -> &Self::Target { &self.peer }
}
impl<'a> ConnectedPeer<'a> {
    /// get the remote tip
    pub fn query_tip(&mut self) -> BlockRef {
        let tip_header = self.connection.get_tip().unwrap();
        BlockRef {
            hash: tip_header.compute_hash(),
            parent: tip_header.get_previous_header(),
            date: tip_header.get_blockdate()
        }
    }

    pub fn send_txaux(mut self, txaux: TxAux) {
        let sent = self.connection.send_transaction(txaux).unwrap();
    }

    pub fn sync(mut self, term: &mut Term) -> Peer<'a> {
        // recover and print the TIP of the network
        let tip = self.query_tip();

        // Start fetching at the current HEAD tag, or the genesis block if
        // it doesn't exist.
        let our_tip = self.load_local_tip();

        let mut best_tip = self.peer.blockchain.load_remote_tips().into_iter().fold(our_tip.clone(), |best_tip, current_tip| {
            if best_tip.0.date < current_tip.0.date {
                current_tip
            } else {
                best_tip
            }
        });

        let mut connection = self.connection;
        let peer = self.peer;

        if best_tip.0.date < tip.date {
            // do nothing, best_tip is behind the remote tip.
        } else if best_tip.0.date > tip.date {
            match storage::block_read(&peer.blockchain.storage, tip.hash.bytes()) {
                None => {
                    // we don't have the block locally... might be a fork, we need to download the
                    // blockchain anyway
                    term.info("remote may have forked from the consensus. Download the blocks anyway.").unwrap();
                    best_tip = our_tip;
                },
                Some(_) => {
                    term.info("remote already as further as it takes").unwrap();
                    peer.save_peer_local_tip(&tip.hash);
                    return peer;
                }
            }
        } else { // best_tip.0.date == tip.date
            if best_tip.0.hash == tip.hash {
                // this is the same block hash. save the local tip
                peer.save_peer_local_tip(&tip.hash);
                return peer;
            } else {
                // it seems the best_tip is for the same date, but has a different hash
                // it could be there is a fork between the remotes.
                //
                // TODO: we might want to drive back to a given block set in the past instead.
                //       in order to avoid re-downloading existing epochs (especially if `our_tip`
                //       is very far in the past).
                best_tip = our_tip;
            }

        }

        // TODO: we need to handle the case where our_tip is not an
        // ancestor of tip. In that case we should start from the last
        // stable epoch before our_tip.

        info!("Fetching from        : {} ({})", best_tip.0.hash, best_tip.0.date);

        // Determine whether the previous epoch is stable yet. Note: This
        // assumes that k is smaller than the number of blocks in an
        // epoch.
        let first_unstable_epoch = tip.date.get_epochid() -
            match tip.date {
                BlockDate::Genesis(_) => 1,
                BlockDate::Normal(d) =>
                    if d.slotid as usize <= peer.blockchain.config.epoch_stability_depth { 1 } else { 0 }
            };
        info!("First unstable epoch : {}", first_unstable_epoch);

        let mut cur_epoch_state : Option<(EpochId, storage::containers::packfile::Writer, SystemTime)> = None;

        let mut last_block : Option<HeaderHash> = None;

        // If our tip is in an epoch that has become stable, we now need
        // to pack it. So read the previously fetched blocks in this epoch
        // and prepend them to the incoming blocks.
        if best_tip.0.date.get_epochid() < first_unstable_epoch && (! best_tip.1) // the second item mark if the tip is genesis
            && !internal::epoch_exists(&peer.blockchain.storage, best_tip.0.date.get_epochid())
        {
            let epoch_id = best_tip.0.date.get_epochid();
            let mut writer = storage::pack::packwriter_init(&peer.blockchain.storage.config);
            let epoch_time_start = SystemTime::now();

            let prev_block = internal::append_blocks_to_epoch_reverse(
                &peer.blockchain.storage, epoch_id, &mut writer, &best_tip.0.hash);

            cur_epoch_state = Some((epoch_id, writer, epoch_time_start));
            last_block = Some(best_tip.0.hash.clone());

            // If tip.slotid < w, the previous epoch won't have been
            // created yet either, so do that now.
            if epoch_id > peer.blockchain.config.epoch_start {
                internal::maybe_create_epoch(&peer.blockchain.storage, epoch_id - 1, &prev_block);
            }
        }

        // If the previous epoch has become stable, then we may need to
        // pack it.
        else if best_tip.0.date.get_epochid() == first_unstable_epoch
            && first_unstable_epoch > peer.blockchain.config.epoch_start
            && !internal::epoch_exists(&peer.blockchain.storage, first_unstable_epoch - 1)
        {
            // Iterate to the last block in the previous epoch.
            let mut cur_hash = best_tip.0.hash.clone();
            loop {
                let block_raw = storage::block_read(&peer.blockchain.storage, cur_hash.bytes()).unwrap();
                let block = block_raw.decode().unwrap();
                let hdr = block.get_header();
                assert!(hdr.get_blockdate().get_epochid() == first_unstable_epoch);
                cur_hash = hdr.get_previous_header();
                if hdr.get_blockdate().is_genesis() { break }
            }
            internal::maybe_create_epoch(&peer.blockchain.storage, first_unstable_epoch - 1, &cur_hash);
        }


        // initialisation of the progress bar:
        let count = tip.date - best_tip.0.date;
        let pbr = term.progress_bar(count as u64);
        connection.get_blocks(&best_tip.0, best_tip.1, &tip, &mut |block_hash, block, block_raw| {
            let date = block.get_header().get_blockdate();
            pbr.inc(1);
            pbr.set_message(&format!("downloading epoch {} -> ", date.get_epochid()));

            // Flush the previous epoch (if any).
            if date.is_genesis() {
                let mut writer_state = None;
                mem::swap(&mut writer_state, &mut cur_epoch_state);
                if let Some((epoch_id, writer, epoch_time_start)) = writer_state {
                    internal::finish_epoch(&peer.blockchain.storage, epoch_id, writer, &epoch_time_start);

                    // Checkpoint the tip so we don't have to refetch
                    // everything if we get interrupted.
                    peer.save_peer_local_tip(last_block.as_ref().unwrap());
                }
            }

            if date.get_epochid() >= first_unstable_epoch {
                // This block is not part of a stable epoch yet and could
                // be rolled back. Therefore we can't pack this epoch
                // yet. Instead we write this block to disk separately.
                let block_hash = storage::types::header_to_blockhash(&block_hash);
                storage::blob::write(&peer.blockchain.storage, &block_hash, block_raw.as_ref()).unwrap();
            } else {

                // If this is the epoch genesis block, start writing a new epoch pack.
                if date.is_genesis() {
                    cur_epoch_state = Some((date.get_epochid(), storage::pack::packwriter_init(&peer.blockchain.storage.config), SystemTime::now()));
                }

                // And append the block to the epoch pack.
                let (_, writer, _) = &mut cur_epoch_state.as_mut().unwrap();
                writer.append(&storage::types::header_to_blockhash(&block_hash), block_raw.as_ref()).unwrap();
            }

            last_block = Some(block_hash.clone());
        }).unwrap();
        pbr.finish_and_clear();

        // Update the tip tag to point to the most recent block.
        if let Some(block_hash) = last_block {
            peer.save_peer_local_tip(&block_hash);
        }

        peer
    }
}

/// a connected peer
pub struct Peer<'a> {
    /// keep a reference to the upper blockchain, we will need to drop
    /// the peer before finalising the blockchain
    pub blockchain: &'a super::Blockchain,

    // we obviously need it in order to connect to a given configuration
    pub config: exe_common::config::net::Peer,

    // the name of the peer
    pub name: String,

    pub tag: String,
}
impl<'a> Peer<'a> {
    pub fn prepare(blockchain: &'a super::Blockchain, name: String) -> Self {
        let config = match blockchain.peers().find(|np| np.name() == &name) {
            None => panic!(""),
            Some(np) => np.peer().clone(),
        };
        let tag = blockchain.mk_remote_tag(&name);

        Peer {
            blockchain,
            name,
            config: config,
            tag
        }
    }

    /// initialise the connection by performing initial handshake (if necessary).
    pub fn connect(self, term: &mut Term) -> Result<ConnectedPeer<'a>, ()> {
        let peer_handshake = exe_common::network::Peer::new(
            self.blockchain.name.clone(),
            self.name.to_owned(),
            self.config.clone(),
            self.blockchain.config.protocol_magic
        );

        let connection = match peer_handshake {
            Err(err) => {
                term.warn(&format!("Unable to initiate handshake with peer {} ({})\n\t{:?}\n", self.name, self.config, err)).unwrap();
                return Err(());
            },
            Ok(peer) => peer
        };

        Ok(ConnectedPeer {
            peer: self,
            connection
        })
    }

    /// load the peer current block
    pub fn load_peer_local_tip(&self) -> HeaderHash {
        match tag::read_hash(&self.blockchain.storage, &self.tag) {
            None => panic!("expecting any peer to have a tag"),
            Some(hh) => hh
        }
    }

    /// save the given peer header hash
    fn save_peer_local_tip(&self, tip: &HeaderHash) {
        tag::write_hash(
            &self.blockchain.storage,
            &self.tag,
            tip
        )
    }

    /// get the remote local tip. the bool is to note if the tip is the same as genesis
    pub fn load_local_tip(&self) -> (BlockRef, bool) {
        let genesis_ref = (BlockRef {
            hash: self.blockchain.config.genesis.clone(),
            parent: self.blockchain.config.genesis_prev.clone(),
            date: BlockDate::Genesis(self.blockchain.config.epoch_start)
        }, true);
        let our_tip = match self.blockchain.storage.get_block_from_tag(&self.tag) {
            Err(storage::Error::NoSuchTag) => genesis_ref,
            Err(err) => panic!(err),
            Ok(block) => {
                let header = block.get_header();
                let hash = header.compute_hash();
                let is_genesis = hash == genesis_ref.0.hash;
                (BlockRef {
                    hash: hash,
                    parent: header.get_previous_header(),
                    date: header.get_blockdate()
                }, is_genesis)
            }
        };
        our_tip
    }
}

mod internal {
    use storage::{self, block_read};
    use cardano::block::{EpochId, HeaderHash};
    use cardano::util::{hex};
    use std::time::{SystemTime, Duration};

    fn duration_print(d: Duration) -> String {
        format!("{}.{:03} seconds", d.as_secs(), d.subsec_millis())
    }


    // Create an epoch from a complete set of previously fetched blocks on
    // disk.
    pub fn maybe_create_epoch(storage: &storage::Storage, epoch_id: EpochId, last_block: &HeaderHash)
    {
        if epoch_exists(&storage, epoch_id) { return }

        info!("Packing epoch {}", epoch_id);

        let mut writer = storage::pack::packwriter_init(&storage.config);
        let epoch_time_start = SystemTime::now();

        append_blocks_to_epoch_reverse(&storage, epoch_id, &mut writer, last_block);

        finish_epoch(storage, epoch_id, writer, &epoch_time_start);

        // TODO: delete the blocks from disk?
    }

    // Check whether an epoch pack exists on disk.
    pub fn epoch_exists(storage: &storage::Storage, epoch_id: EpochId) -> bool
    {
        // FIXME: epoch_read() is a bit inefficient here; we really only
        // want to know if it exists.
        storage::epoch::epoch_read(&storage.config, epoch_id).is_ok()
    }

    pub fn append_blocks_to_epoch_reverse(
        storage: &storage::Storage,
        epoch_id : EpochId,
        writer : &mut storage::containers::packfile::Writer,
        last_block: &HeaderHash)
        -> HeaderHash
    {
        let mut cur_hash = last_block.clone();
        let mut blocks = vec![];
        loop {
            let block_raw = block_read(&storage, cur_hash.bytes()).unwrap();
            let block = block_raw.decode().unwrap();
            let hdr = block.get_header();
            assert!(hdr.get_blockdate().get_epochid() == epoch_id);
            blocks.push((storage::types::header_to_blockhash(&cur_hash), block_raw));
            cur_hash = hdr.get_previous_header();
            if hdr.get_blockdate().is_genesis() { break }
        }

        while let Some((hash, block_raw)) = blocks.pop() {
            writer.append(&hash, block_raw.as_ref()).unwrap();
        }

        cur_hash
    }

    pub fn finish_epoch(storage: &storage::Storage, epoch_id : EpochId, writer : storage::containers::packfile::Writer, epoch_time_start : &SystemTime)
    {
        let (packhash, index) = storage::pack::packwriter_finalize(&storage.config, writer);
        let (_, tmpfile) = storage::pack::create_index(&storage, &index);
        tmpfile.render_permanent(&storage.config.get_index_filepath(&packhash)).unwrap();
        let epoch_time_elapsed = epoch_time_start.elapsed().unwrap();

        // TODO: should test that epoch <epoch_id - 1> exists.

        storage::epoch::epoch_create(&storage.config, &packhash, epoch_id);

        info!( "=> pack {} written for epoch {} in {}"
             , hex::encode(&packhash[..])
             , epoch_id, duration_print(epoch_time_elapsed)
             );
    }
}
