#[derive(Clone, Copy)]
pub struct Config {
    pub port: &'static str,
    pub bind_addr: &'static str,
    pub aws_endpoint_url: &'static str,
    pub aws_region: &'static str,
    pub aws_access_key_id: &'static str,
    pub aws_secret_access_key: &'static str,
    pub aws_s3_bucket: &'static str,
}

impl Config {
    pub fn new() -> Self {
        Self {
            port: "3000",
            bind_addr: "0.0.0.0",
            aws_endpoint_url: "http://127.0.0.1:4567",
            aws_region: "ap-northeast-1",
            aws_access_key_id: "AAAAAAAAAAAAAAAAAAAA",
            aws_secret_access_key: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
            aws_s3_bucket: "local-test",
        }
    }
}
