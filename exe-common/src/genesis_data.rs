use cardano::block::HeaderHash;
use std::str::FromStr;

pub fn get_genesis_data(genesis_prev: &HeaderHash) -> Result<&str, HeaderHash> {
    if genesis_prev
        == &HeaderHash::from_str("5f20df933584822601f9e3f8c024eb5eb252fe8cefb24d1317dc3d432e940ebb")
            .unwrap()
    {
        Ok(include_str!(
            "../genesis/5f20df933584822601f9e3f8c024eb5eb252fe8cefb24d1317dc3d432e940ebb.json"
        ))
    } else if genesis_prev
        == &HeaderHash::from_str("b7f76950bc4866423538ab7764fc1c7020b24a5f717a5bee3109ff2796567214")
            .unwrap()
    {
        Ok(include_str!(
            "../genesis/b7f76950bc4866423538ab7764fc1c7020b24a5f717a5bee3109ff2796567214.json"
        ))
    } else if genesis_prev
        == &HeaderHash::from_str("c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323")
            .unwrap()
    {
        Ok(include_str!(
            "../genesis/c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323.json"
        ))
    } else {
        Err(genesis_prev.clone())
    }
}
