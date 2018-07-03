/// macro to efficiently serialise the given structure into
/// cbor binary.
///
/// This performs an in memory serialisation and returns the
/// buffer wrapped in a [`Result`](../enum.Result.html).
///
#[macro_export]
macro_rules! cbor {
    ($x:expr) => ({
        ::cbor_event::se::Serializer::new_vec()
            .serialize(& $x)
            .map(|s| s.finalize())
    });
}
