use super::super::config::web;
use reqwest::StatusCode;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    pub fn new(cfg: web::Config) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(cfg.user_agent)
            .timeout(Duration::from_secs(cfg.timeout));
        Self {
            http: client.build().expect("failed to build http client"),
        }
    }

    pub async fn get(&self, url: String) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        // https://docs.rs/reqwest/latest/reqwest/struct.Client.html
        match self.http.get(&url).send().await {
            // https://docs.rs/reqwest/latest/reqwest/struct.Response.html
            Ok(response) => {
                if response.status() == StatusCode::NOT_FOUND {
                    return Ok(None);
                }
                if !response.status().is_success() {
                    return Err(Box::from(format!("GET {}: {}", &url, response.status())));
                }
                let bytes = response.bytes().await?;
                Ok(Some(bytes.to_vec()))
            }
            // https://docs.rs/reqwest/latest/reqwest/struct.Error.html
            Err(err) => {
                if err.status() == Some(StatusCode::NOT_FOUND) {
                    return Ok(None);
                }
                Err(Box::from(err))
            }
        }
    }
}
