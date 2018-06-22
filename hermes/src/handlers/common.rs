
use cardano::block::EpochId;

pub fn validate_network_name(v: &&str) -> bool {
    v.chars().all(|c| c.is_ascii_alphanumeric())
}

pub fn validate_epochid(v: &&str) -> Option<EpochId> {
    if ! v.chars().all(|c| c.is_digit(10)) {
        None
    } else {
        Some(v.parse::<EpochId>().unwrap())
    }
}
