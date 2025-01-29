use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub user_agent: String,
    pub timeout: u64,
}
