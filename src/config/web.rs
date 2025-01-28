#[derive(Clone, Copy)]
pub struct Config {
    pub user_agent: &'static str,
    pub timeout: u64,
}

impl Config {
    pub fn new() -> Self {
        Self {
            user_agent: "fanlin/0.0.1",
            timeout: 5u64,
        }
    }
}
