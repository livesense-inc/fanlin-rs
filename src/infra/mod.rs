pub mod s3;

use super::config;

pub struct Client {
    pub s3: s3::Client,
}

impl Client {
    pub async fn new(cfg: &config::Config) -> Self {
        Self {
            s3: s3::Client::new(cfg).await,
        }
    }
}
