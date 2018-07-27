//! local blockchain configuration related functions and tools
//!

use std::path::PathBuf;

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
