#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account(String);
impl Account {
    pub fn new(name: String) -> Self { Account(name) }
}
impl Default for Account {
    fn default() -> Self { Account("Main".to_string()) }
}
