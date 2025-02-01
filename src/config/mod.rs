pub mod s3;
pub mod web;

use serde::Deserialize;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Clone, Debug, Deserialize)]
pub struct Provider {
    pub path: String,
    pub src: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Client {
    pub s3: s3::Config,
    pub web: web::Config,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub port: usize,
    pub bind_addr: String,
    pub client: Client,
    pub providers: Vec<Provider>,
}

impl Config {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
        // https://docs.rs/serde_json/latest/serde_json/fn.from_reader.html
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let cfg = serde_json::from_reader(reader)?;
        Ok(cfg)
    }
}
