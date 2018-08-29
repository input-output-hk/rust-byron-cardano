use std::path::PathBuf;
use std::io::{Write};

use exe_common::config::net::Config;
use storage;

use utils::term::Term;

use super::peer;
use super::Blockchain;
use cardano::{self, block::{RawBlock}};

/// function to create and initialize a given new blockchain
///
/// It will mainly create the subdirectories needed for the storage
/// of blocks, epochs and tags.
///
/// If the given blockchain configuration provides some preset peers
/// each peer will be initialized with an associated tag pointing to
/// the genesis hash of the blockchain (given in the same configuration
/// structure `Config`).
///
pub fn new( mut term: Term
          , root_dir: PathBuf
          , name: String
          , config: Config
          )
{
    let blockchain = Blockchain::new(root_dir, name.clone(), config);
    blockchain.save();

    term.success(&format!("local blockchain `{}' created.\n", &name)).unwrap();
}

pub fn list( mut term: Term
           , root_dir: PathBuf
           , detailed: bool
           )
{
    let blockchains_dir = super::config::blockchains_directory(&root_dir);
    for entry in ::std::fs::read_dir(blockchains_dir).unwrap() {
        let entry = entry.unwrap();
        if ! entry.file_type().unwrap().is_dir() {
            term.warn(&format!("unexpected file in blockchains directory: {:?}", entry.path())).unwrap();
            continue;
        }
        let name = entry.file_name().into_string().unwrap_or_else(|err| {
            panic!("invalid utf8... {:?}", err)
        });

        let blockchain = Blockchain::load(root_dir.clone(), name);

        term.info(&blockchain.name).unwrap();
        if detailed {
            let (tip, _is_genesis) = blockchain.load_tip();
            let tag_path = blockchain.dir.join("tag").join(super::LOCAL_BLOCKCHAIN_TIP_TAG);
            let metadata = ::std::fs::metadata(tag_path).unwrap();
            let now = ::std::time::SystemTime::now();
            let fetched_date = metadata.modified().unwrap();
            let fetched_since = ::std::time::Duration::new(now.duration_since(fetched_date).unwrap().as_secs(), 0);

            term.simply("\t").unwrap();
            term.success(&format!("{} ({})", tip.hash, tip.date)).unwrap();
            term.simply("\t").unwrap();
            term.warn(&format!("(updated {} ago)", format_duration(fetched_since))).unwrap();
        }
        term.simply("\n").unwrap();
    }
}

pub fn destroy( mut term: Term
              , root_dir: PathBuf
              , name: String
              )
{
    let blockchain = Blockchain::load(root_dir, name);

    writeln!(term, "You are about to destroy the local blockchain {}.
This means that all the blocks downloaded will be deleted and that the attached
wallets won't be able to interact with this blockchain.",
        ::console::style(&blockchain.name).bold().red(),
    ).unwrap();

    let confirmation = ::dialoguer::Confirmation::new("Are you sure?")
        .use_line_input(true)
        .clear(false)
        .default(false)
        .interact().unwrap();
    if ! confirmation { ::std::process::exit(0); }

    unsafe { blockchain.destroy() }.unwrap();

    term.success("blockchain successfully destroyed").unwrap();
}

/// function to add a remote to the given blockchain
///
/// It will create the appropriate tag referring to the blockchain
/// genesis hash. This is because when add a new peer we don't assume
/// anything more than the genesis block.
///
pub fn remote_add( mut term: Term
                 , root_dir: PathBuf
                 , name: String
                 , remote_alias: String
                 , remote_endpoint: String
                 )
{
    let mut blockchain = Blockchain::load(root_dir, name);
    blockchain.add_peer(remote_alias.clone(), remote_endpoint);
    blockchain.save();

    term.success(&format!("remote `{}' node added to blockchain `{}'\n", remote_alias, blockchain.name)).unwrap();
}

/// remove the given peer from the blockchain
///
/// it will also delete all the metadata associated to this peer
/// such as the tag pointing to the remote's tip.
///
pub fn remote_rm( mut term: Term
                , root_dir: PathBuf
                , name: String
                , remote_alias: String
                )
{
    let mut blockchain = Blockchain::load(root_dir, name);
    blockchain.remove_peer(remote_alias.clone());
    blockchain.save();

    term.success(&format!("remote `{}' node removed from blockchain `{}'\n", remote_alias, blockchain.name)).unwrap();
}

pub fn remote_fetch( mut term: Term
                   , root_dir: PathBuf
                   , name: String
                   , peers: Vec<String>
                   )
{
    let blockchain = Blockchain::load(root_dir, name);

    for np in blockchain.peers() {
        if peers.is_empty() || peers.contains(&np.name().to_owned()) {
            term.info(&format!("fetching blocks from peer: {}\n", np.name())).unwrap();

            let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());

            peer.connect(&mut term).unwrap().sync(&mut term);
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum RemoteDetail {
    Short,
    Local,
    Remote
}

pub fn remote_ls( mut term: Term
                , root_dir: PathBuf
                , name: String
                , detailed: RemoteDetail
                )
{
    let blockchain = Blockchain::load(root_dir, name);

    for np in blockchain.peers() {
        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());
        let (tip, _is_genesis) = peer.load_local_tip();

        term.info(&format!("{}", peer.name)).unwrap();
        term.simply(" (").unwrap();
        term.success(&format!("{}", peer.config)).unwrap();
        term.simply(")\n").unwrap();

        if detailed >= RemoteDetail::Local {
            let tag_path = blockchain.dir.join("tag").join(&peer.tag);
            let metadata = ::std::fs::metadata(tag_path).unwrap();
            let now = ::std::time::SystemTime::now();
            let fetched_date = metadata.modified().unwrap();
            // get the difference between now and the last fetch, only keep up to the seconds
            let fetched_since = ::std::time::Duration::new(now.duration_since(fetched_date).unwrap().as_secs(), 0);

            term.simply(" * last fetch:      ").unwrap();
            term.info(&format!("{} ({} ago)", format_systemtime(fetched_date), format_duration(fetched_since))).unwrap();
            term.simply("\n").unwrap();
            term.simply(" * local tip hash:  ").unwrap();
            term.success(&format!("{}", tip.hash)).unwrap();
            term.simply("\n").unwrap();
            term.simply(" * local tip date:  ").unwrap();
            term.success(&format!("{}", tip.date)).unwrap();
            term.simply("\n").unwrap();

            if detailed >= RemoteDetail::Remote {
                let mut connected_peer = peer.connect(&mut term).unwrap();
                let remote_tip = connected_peer.query_tip();
                let block_diff = remote_tip.date - tip.date;

                term.simply(" * remote tip hash: ").unwrap();
                term.warn(&format!("{}", remote_tip.hash)).unwrap();
                term.simply("\n").unwrap();
                term.simply(" * remote tip date: ").unwrap();
                term.warn(&format!("{}", remote_tip.date)).unwrap();
                term.simply("\n").unwrap();
                term.simply(" * local is ").unwrap();
                term.warn(&format!("{}", block_diff)).unwrap();
                term.simply(" blocks behind remote\n").unwrap();
            }
        }
    }
}

fn format_systemtime(time: ::std::time::SystemTime) -> String {
    format!("{}", ::humantime::format_rfc3339(time)).chars().take(10).collect()
}
fn format_duration(duration: ::std::time::Duration) -> String {
    format!("{}", ::humantime::format_duration(duration))
}

pub fn log( mut term: Term
          , root_dir: PathBuf
          , name: String
          , from: Option<String>
          )
{
    let blockchain = Blockchain::load(root_dir, name);

    let from = if let Some(hash_hex) = from {
        let hash = super::config::parse_block_hash(&mut term, &hash_hex);

        if storage::block_location(&blockchain.storage, hash.bytes()).is_none() {
            term.error(&format!("block hash `{}' is not present in the local blockchain\n", hash_hex)).unwrap();
            ::std::process::exit(1);
        }

        hash
    } else {
        blockchain.load_tip().0.hash
    };

    for block in storage::block::iter::ReverseIter::from(&blockchain.storage, from).unwrap() {
        use utils::pretty::Pretty;

        block.pretty(&mut term, 0).unwrap();
    }
}

pub fn forward( mut term: Term
              , root_dir: PathBuf
              , name: String
              , to: Option<String>
              )
{
    let blockchain = Blockchain::load(root_dir, name);

    let hash = if let Some(hash_hex) = to {
        let hash = super::config::parse_block_hash(&mut term, &hash_hex);

        if ::storage::block_location(&blockchain.storage, hash.bytes()).is_none() {
            term.error(&format!("block hash `{}' is not present in the local blockchain\n", hash_hex)).unwrap();
            ::std::process::exit(1);
        }

        hash
    } else {
        let initial_tip = blockchain.load_tip().0;

        let tip = blockchain.peers().map(|np| {
            peer::Peer::prepare(&blockchain, np.name().to_owned()).load_local_tip().0
        }).fold(initial_tip, |current_tip, tip| {
            if tip.date > current_tip.date {
                tip
            } else {
                current_tip
            }
        });

        tip.hash
    };

    term.success(&format!("forward local tip to: {}\n", hash)).unwrap();

    blockchain.save_tip(&hash)
}

pub fn pull( mut term: Term
           , root_dir: PathBuf
           , name: String
           )
{
    let blockchain = Blockchain::load(root_dir.clone(), name.clone());

    for np in blockchain.peers() {
        if ! np.is_native() { continue; }
        term.info(&format!("fetching blocks from peer: {}\n", np.name())).unwrap();

        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());

        peer.connect(&mut term).unwrap().sync(&mut term);
    }

    forward(term, root_dir, name, None)
}

fn get_block(mut term: &mut Term, blockchain: &Blockchain, hash_str: &str) -> RawBlock
{
    let hash = super::config::parse_block_hash(&mut term, &hash_str);
    let block_location = match ::storage::block_location(&blockchain.storage, hash.bytes()) {
        None => {
            term.error(&format!("block hash `{}' is not present in the local blockchain\n", hash_str)).unwrap();
            ::std::process::exit(1);
        },
        Some(loc) => loc
    };

    debug!("blk location: {:?}", block_location);

    match ::storage::block_read_location(&blockchain.storage, &block_location, hash.bytes()) {
        None        => {
            // this is a bug, we have a block location available for this hash
            // but we were not able to read the block.
            panic!("the impossible happened, we have a block location of this given block `{}'", hash)
        },
        Some(rblk) => rblk
    }
}

pub fn cat( mut term: Term
          , root_dir: PathBuf
          , name: String
          , hash_str: &str
          , no_parse: bool
          , debug: bool
          )
{
    let blockchain = Blockchain::load(root_dir.clone(), name.clone());
    let rblk = get_block(&mut term, &blockchain, hash_str);

    if no_parse {
        ::std::io::stdout().write(rblk.as_ref()).unwrap();
        ::std::io::stdout().flush().unwrap();
    } else {
        use utils::pretty::Pretty;

        let blk = rblk.decode().unwrap();
        if debug {
            writeln!(term, "{:#?}", blk).unwrap();
        } else {
            blk.pretty(&mut term, 0).unwrap();
        }
    }
}

pub fn status( mut term: Term
         , root_dir: PathBuf
         , name: String
         )
{
    let blockchain = Blockchain::load(root_dir, name);

    term.warn("Blockchain:\n").unwrap();
    {
        let (tip, _is_genesis) = blockchain.load_tip();
        let tag_path = blockchain.dir.join("tag").join(super::LOCAL_BLOCKCHAIN_TIP_TAG);
        let metadata = ::std::fs::metadata(tag_path).unwrap();
        let now = ::std::time::SystemTime::now();
        let fetched_date = metadata.modified().unwrap();
        // get the difference between now and the last fetch, only keep up to the seconds
        let fetched_since = ::std::time::Duration::new(now.duration_since(fetched_date).unwrap().as_secs(), 0);

        term.simply("   * last forward:    ").unwrap();
        term.info(&format!("{} ({} ago)", format_systemtime(fetched_date), format_duration(fetched_since))).unwrap();
        term.simply("\n").unwrap();
        term.simply("   * local tip hash:  ").unwrap();
        term.success(&format!("{}", tip.hash)).unwrap();
        term.simply("\n").unwrap();
        term.simply("   * local tip date:  ").unwrap();
        term.success(&format!("{}", tip.date)).unwrap();
        term.simply("\n").unwrap();
    }

    term.warn("Peers:\n").unwrap();
    for (idx, np) in blockchain.peers().enumerate() {
        let peer = peer::Peer::prepare(&blockchain, np.name().to_owned());
        let (tip, _is_genesis) = peer.load_local_tip();

        term.info(&format!(" {}. {}", idx + 1, peer.name)).unwrap();
        term.simply(" (").unwrap();
        term.success(&format!("{}", peer.config)).unwrap();
        term.simply(")\n").unwrap();

        let tag_path = blockchain.dir.join("tag").join(&peer.tag);
        let metadata = ::std::fs::metadata(tag_path).unwrap();
        let now = ::std::time::SystemTime::now();
        let fetched_date = metadata.modified().unwrap();
        // get the difference between now and the last fetch, only keep up to the seconds
        let fetched_since = ::std::time::Duration::new(now.duration_since(fetched_date).unwrap().as_secs(), 0);

        term.simply("     * last fetch:      ").unwrap();
        term.info(&format!("{} ({} ago)", format_systemtime(fetched_date), format_duration(fetched_since))).unwrap();
        term.simply("\n").unwrap();
        term.simply("     * local tip hash:  ").unwrap();
        term.success(&format!("{}", tip.hash)).unwrap();
        term.simply("\n").unwrap();
        term.simply("     * local tip date:  ").unwrap();
        term.success(&format!("{}", tip.date)).unwrap();
        term.simply("\n").unwrap();
    }
}

pub fn verify_block( mut term: Term
                   , root_dir: PathBuf
                   , name: String
                   , hash_str: &str
                   )
{
    let blockchain = Blockchain::load(root_dir, name);
    let hash = super::config::parse_block_hash(&mut term, &hash_str);
    let rblk = get_block(&mut term, &blockchain, hash_str);
    match rblk.decode() {
        Ok(blk) => {
            match cardano::block::verify_block(blockchain.config.protocol_magic, &hash, &blk) {
                Ok(()) => {
                    term.success("Ok").unwrap();
                    term.simply("\n").unwrap();
                }
                Err(err) => {
                    term.error("Error: ").unwrap();
                    term.simply(&format!("{:?}", err)).unwrap();
                    term.simply("\n").unwrap();
                    ::std::process::exit(1);
                }
            };
        },
        Err(err) => {
            term.error("Error: ").unwrap();
            term.simply(&format!("{:?}", err)).unwrap();
            term.simply("\n").unwrap();
            ::std::process::exit(1);
        }
    }
}

pub fn verify_chain( mut term: Term
                   , root_dir: PathBuf
                   , name: String
                   )
{
    let blockchain = Blockchain::load(root_dir, name);

    let mut bad_blocks = 0;
    let mut nr_blocks = 0;

    for rblk in blockchain.iter_to_tip(blockchain.config.genesis.clone()).unwrap() {
        nr_blocks += 1;
        // FIXME: inefficient - the iterator has already decoded the block.
        let rblk = rblk.unwrap();
        let blk = rblk.decode().unwrap();
        let hash = blk.get_header().compute_hash();
        writeln!(term, "block {} {}", hash, blk.get_header().get_blockdate()).unwrap();
        match cardano::block::verify_block(blockchain.config.protocol_magic, &hash, &blk) {
            Ok(()) => {},
            Err(err) => {
                bad_blocks += 1;
                term.error(&format!("Block {} ({}) is invalid: {:?}", hash, blk.get_header().get_blockdate(), err)).unwrap();
                term.simply("\n").unwrap();
                //::std::process::exit(1);
            }
        }
    }

    if bad_blocks > 0 {
        term.error(&format!("{} out of {} blocks are invalid", bad_blocks, nr_blocks)).unwrap();
        term.simply("\n").unwrap();
        ::std::process::exit(1);
    }

    term.success(&format!("All {} blocks are valid", nr_blocks)).unwrap();
    term.simply("\n").unwrap();
}
