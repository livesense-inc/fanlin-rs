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
            s3: s3::Client::new(
                cfg.client.s3.aws_endpoint_url.as_str(),
                cfg.client.s3.aws_region.as_str(),
                cfg.client.s3.aws_access_key_id.as_str(),
                cfg.client.s3.aws_secret_access_key.as_str(),
            )
            .await,
            web: web::Client::new(
                cfg.client.web.user_agent.as_str(),
                cfg.client.web.timeout as u64,
            ),
        }
    }
}
