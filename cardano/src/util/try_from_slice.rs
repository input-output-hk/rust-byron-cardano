/// type conversion to work along side objects that implements `AsRef<[u8]>`
///
/// The idea is to facilitate serialization/deserialization implementation for
/// generic types by providing a simple common interface between the objects.
///
/// This trait is there as a replacement to [`TryFrom`] which is not yet stable.
///
/// [`TryFrom`]: https://doc.rust-lang.org/std/convert/trait.TryFrom.html
///
pub trait TryFromSlice: Sized {
    /// the error kind. We expect Display for now so we don't have to
    /// implement all instances yet to all the Error kinds but this
    /// will soon be extended to require the `Error` trait
    type Error: ::std::error::Error;

    /// attempt to construct the object from the given slice
    fn try_from_slice(slice: &[u8]) -> Result<Self, Self::Error>;
}
