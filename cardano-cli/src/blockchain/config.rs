//! local blockchain configuration related functions and tools
//!

use std::path::PathBuf;
use utils::term::Term;

/// this is the name of the directory where the blockchains'
/// blocks, epochs and tags will lie.
pub const BLOCKCHAINS_DIRECTORY : &'static str = "blockchains";

/// handy function to define where to find the blockchains related
/// functions in a given _cardano-cli_ directory.
///
pub fn directory( root_dir: PathBuf
                , name: &str
                ) -> PathBuf
{
    root_dir.join(BLOCKCHAINS_DIRECTORY).join(name)
}

/// function to check if the given string is a valid block hash
///
/// This function will print an error message if the given hash is not
/// hexadecimal or is not a valid block hash.
///
pub fn parse_block_hash(term: &mut Term, hash_str: &str) -> ::cardano::block::HeaderHash {
    match ::cardano::util::hex::decode(&hash_str) {
        Ok(hash) => match ::cardano::block::HeaderHash::from_slice(hash.as_ref()) {
            Err(err) => {
                debug!("invalid block hash: {}", err);
                term.error(&format!("invalid hash `{}': this is not a valid block hash\n", hash_str)).unwrap();
                ::std::process::exit(1);
            },
            Ok(hash) => hash
        },
        Err(err) => {
            debug!("invalid block hash: {:?}", err);
            term.error(&format!("invalid hash `{}': invalid hexadecimal\n", hash_str)).unwrap();
            ::std::process::exit(1);
        }
    }
}
