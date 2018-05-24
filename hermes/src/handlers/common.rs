
pub fn validate_network_name(v: &&str) -> bool {
    v.chars().all(|c| c.is_ascii_alphanumeric())
}
