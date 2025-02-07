pub mod file;
pub mod s3;
pub mod web;

use super::config;

#[derive(Clone, Debug)]
pub struct Client {
    pub s3: s3::Client,
    pub web: web::Client,
    pub file: file::Client,
}

impl Client {
    pub async fn new(cfg: &config::Config) -> Self {
        Self {
            s3: s3::Client::new(cfg.client.s3.clone()).await,
            web: web::Client::new(cfg.client.web.clone()),
            file: file::Client::new(),
        }
    }
}

#[cfg(test)]
impl Client {
    pub async fn for_test() -> Self {
        Self {
            s3: s3::Client::for_test().await,
            web: web::Client::for_test(),
            file: file::Client::for_test(),
        }
    }
}
