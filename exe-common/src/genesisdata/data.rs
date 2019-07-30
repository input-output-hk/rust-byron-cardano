use cardano::block::HeaderHash;
use std::str::FromStr;

pub fn get_genesis_data(genesis_prev: &HeaderHash) -> Result<&str, HeaderHash> {
    if genesis_prev
        == &HeaderHash::from_str("5f20df933584822601f9e3f8c024eb5eb252fe8cefb24d1317dc3d432e940ebb")
            .unwrap()
    {
        Ok(include_str!(
            "../../genesis/5f20df933584822601f9e3f8c024eb5eb252fe8cefb24d1317dc3d432e940ebb.json"
        ))
    } else if genesis_prev
        == &HeaderHash::from_str("96fceff972c2c06bd3bb5243c39215333be6d56aaf4823073dca31afe5038471")
            .unwrap()
    {
        Ok(include_str!(
            "../../genesis/96fceff972c2c06bd3bb5243c39215333be6d56aaf4823073dca31afe5038471.json"
        ))
    } else if genesis_prev
        == &HeaderHash::from_str("c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323")
            .unwrap()
    {
        Ok(include_str!(
            "../../genesis/c6a004d3d178f600cd8caa10abbebe1549bef878f0665aea2903472d5abf7323.json"
        ))
    } else {
        Err(genesis_prev.clone())
    }
}
