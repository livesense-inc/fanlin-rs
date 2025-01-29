use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub aws_region: String,
    pub aws_endpoint_url: Option<String>,
    pub aws_access_key_id: Option<String>,
    pub aws_secret_access_key: Option<String>,
}
