/// CBOR len: either a fixed size or an indefinite length.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Len {
    Indefinite,
    Len(u64)
}
impl Len {
    pub fn is_null(&self) -> bool {
        match self {
            Len::Len(0) => true,
            _           => false
        }
    }
    pub fn non_null(self) -> Option<Self> {
        if self.is_null() { None } else { Some(self) }
    }

    pub fn indefinite(&self) -> bool {
        self == &Len::Indefinite
    }
}
