#[macro_export]
macro_rules! cbor {
    ($x:expr) => ({
        ::raw_cbor::se::Serializer::new()
            .serialize(& $x)
            .map(|s| s.finalize())
    });
}
