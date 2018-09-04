use std::path::{PathBuf};
use super::{StagingId};

const TRANSACTION_DIR : &'static str = "transactions";

/// return the directory path where all the pending transactions are
pub fn transaction_directory(root_dir: PathBuf) -> PathBuf {
    root_dir.join(TRANSACTION_DIR)
}

/// get the path of the given transaction via its staging id
pub fn transaction_file(root_dir: PathBuf, id: StagingId) -> PathBuf {
    transaction_directory(root_dir).join(id.to_string())
}
