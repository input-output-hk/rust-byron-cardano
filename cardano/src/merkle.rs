use cbor_event::se;
use hash::Blake2b256;

pub type Hash = Blake2b256;

pub enum MerkleTree {
    Empty,
    Tree(usize, MerkleNode),
}

pub enum MerkleNode {
    Branch(Hash, Box<MerkleNode>, Box<MerkleNode>),
    Leaf(Hash),
}

impl MerkleTree {
    pub fn new<T>(xs: &[T]) -> Self
    where
        T: se::Serialize,
    {
        if xs.is_empty() {
            return MerkleTree::Empty;
        }

        MerkleTree::Tree(xs.len(), MerkleNode::make_tree(xs))
    }

    pub fn get_root_hash(&self) -> Hash {
        match self {
            MerkleTree::Empty => Hash::new(&vec![]), // FIXME: cache
            MerkleTree::Tree(_, node) => node.get_root_hash().clone(),
        }
    }
}

impl MerkleNode {
    fn make_tree<T>(xs: &[T]) -> Self
    where
        T: se::Serialize,
    {
        if xs.is_empty() {
            panic!("make_tree applied to empty list")
        } else if xs.len() == 1 {
            let bs = vec![0u8];
            let mut se = se::Serializer::new(bs);
            xs[0].serialize(&mut se).unwrap();
            MerkleNode::Leaf(Hash::new(&se.finalize()))
        } else {
            let i = xs.len().checked_next_power_of_two().unwrap() >> 1;
            let a = MerkleNode::make_tree(&xs[0..i]);
            let b = MerkleNode::make_tree(&xs[i..]);
            let mut bs = vec![1u8];
            bs.extend(a.get_root_hash().as_hash_bytes());
            bs.extend(b.get_root_hash().as_hash_bytes());
            MerkleNode::Branch(Hash::new(&bs), Box::new(a), Box::new(b))
        }
    }

    fn get_root_hash(&self) -> &Hash {
        match self {
            MerkleNode::Branch(hash, _, _) => hash,
            MerkleNode::Leaf(hash) => hash,
        }
    }
}
