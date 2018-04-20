use hdpayload::{Path};

const BIP44_PURPOSE   : u32 = 0x8000002C;
const BIP44_COIN_TYPE : u32 = 0x80000717;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Addressing {
    account: u32,
    change: u32,
    index: u32,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum AddrType {
    Internal,
    External,
}

impl Addressing {
    pub fn new(account: u32, typ: AddrType) -> Self {
        let change = match typ {
                        AddrType::Internal => 0,
                        AddrType::External => 1,
                    };
        Addressing { account: 0x80000000 | account, change: change, index: 0 }
    }

    pub fn to_path(&self) -> Path {
        Path::new(vec![BIP44_PURPOSE, BIP44_COIN_TYPE, self.account, self.change, self.index])
    }

    pub fn incr(&self, incr: u32) -> Self {
        let mut addr = self.clone();
        addr.index += incr;
        addr
    }

    pub fn next_chunks(&self, chunk_size: usize) -> Vec<Self> {
        let mut v = Vec::with_capacity(chunk_size);
        for i in 0..chunk_size {
            let r = self.incr(i as u32);
            v.push(r);
        }
        v
    }
}
