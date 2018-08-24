use storage::{containers::append, utils::lock::{self, Lock}, utils::tmpfile::TmpFile, utils::serialize::{SIZE_SIZE, write_size}};
use cardano::util::base58;
use rand;
use std::path::PathBuf;
use std::io;

#[derive(Debug, Clone, Copy)]
pub struct StagingId(u32);

impl StagingId {
    pub fn generate() -> Self {
        StagingId(rand::random())
    }

    pub fn as_string(self) -> String {
        let mut buf = [0u8;SIZE_SIZE];
        write_size(&mut buf, self.0);
        base58::encode(&buf)
    }
}

pub enum Operation {
    // TODO add parameters
    AddInput,
    AddOutput,
    RemoveInput,
    RemoveOutput,
}

impl Operation {
    pub fn serialize(self) -> Vec<u8> {
        unimplemented!()
    }
}

pub struct Transaction {
    id: StagingId,
    operations: Vec<Operation>,
}

fn get_transaction_path(transaction_dir: &mut PathBuf, id: StagingId) -> PathBuf {
    // TODO ugly path append
    transaction_dir.push(id.as_string());
    transaction_dir.clone()
}

const MAGIC_TRANSACTION_V1 : &'static [u8] = b"TRANSACTION_V1";

impl Transaction {
    pub fn new(transaction_dir: PathBuf, id: StagingId) -> append::Result<Self> {
        let path = get_transaction_path(&mut transaction_dir.clone(), id);
        let lock = Lock::lock(path)?;
        let mut w = append::Writer::open(lock)?;
        w.append_bytes(MAGIC_TRANSACTION_V1)?;
        Ok(Transaction { id: id, operations: Vec::new() })
    }

    pub fn read_from_file(transaction_dir: PathBuf, id: StagingId) -> append::Result<(Self, Lock)> {
        let path = get_transaction_path(&mut transaction_dir.clone(), id);
        let lock = Lock::lock(path)?;
        let mut r = append::Reader::open(lock)?;
        let magic_got = r.next()?;
        match magic_got {
            None => {},
            Some(magic_got) => {
                if magic_got != MAGIC_TRANSACTION_V1 {
                    return Err(append::Error::EOF);
                }
            },
        }

        let mut operations = Vec::new();

        loop {
            match r.next()? {
                None => break,
                Some(o) => {
                    // TODO append operations
                },
            }
        }
        let lock_ret = r.close();

        let transaction = Transaction { id : id, operations : operations };
        Ok((transaction, lock_ret))
    }

    pub fn append(&self, transaction_dir: PathBuf, transaction_op: Operation) -> append::Result<()> {
        let path = get_transaction_path(&mut transaction_dir.clone(), self.id);
        let lock = Lock::lock(path).unwrap();
        let mut w = append::Writer::open(lock)?;

        w.append_bytes(&transaction_op.serialize())?;

        w.close();

        Ok(())
    }
}