pub mod s3;
pub mod web;

use super::config;

pub struct Client {
    pub s3: s3::Client,
    pub web: web::Client,
}

impl Client {
    pub async fn new(cfg: &config::Config) -> Self {
        Self {
            s3: s3::Client::new(cfg.client.s3.clone()).await,
            web: web::Client::new(cfg.client.web.clone()),
        }
    }
}
