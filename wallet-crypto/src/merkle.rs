use std::marker::PhantomData;

type Hash = Vec<u8>;
pub struct Root<T> {
    hash: Hash,
    _phantom: PhantomData<T>,
}

pub enum Node<T: Sized> {
    Leaf(Hash, T),
    Branch(Box<Node<T>>, Root<T>, Box<Node<T>>),
}
