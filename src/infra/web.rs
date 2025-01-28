use reqwest::StatusCode;
use std::time::Duration;

pub struct Client {
    http: reqwest::Client,
}

impl Client {
    pub fn new(ua: &str, timeout: u64) -> Self {
        let client = reqwest::ClientBuilder::new()
            .user_agent(ua)
            .timeout(Duration::from_secs(timeout));
        Self {
            http: client.build().unwrap(),
        }
    }

    pub async fn get_image<'a>(
        &self,
        url: &'a str,
    ) -> Option<Result<Vec<u8>, Box<dyn std::error::Error>>> {
        // https://docs.rs/reqwest/latest/reqwest/struct.Client.html
        match self.http.get(url).send().await {
            // https://docs.rs/reqwest/latest/reqwest/struct.Response.html
            Ok(response) => {
                if response.status() == StatusCode::NOT_FOUND {
                    return None;
                }
                match response.bytes().await {
                    Ok(bytes) => Some(Ok(bytes.to_vec())),
                    Err(err) => Some(Err(Box::from(err))),
                }
            }
            // https://docs.rs/reqwest/latest/reqwest/struct.Error.html
            Err(err) => {
                if err.status() == Some(StatusCode::NOT_FOUND) {
                    return None;
                }
                Some(Err(Box::from(err)))
            }
        }
    }
}
