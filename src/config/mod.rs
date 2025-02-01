pub mod s3;
pub mod web;

use serde::Deserialize;
use std::fs::File;
use std::io::{BufReader, Read};
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
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Config, Box<dyn std::error::Error>> {
        // https://docs.rs/serde_json/latest/serde_json/fn.from_reader.html
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    pub fn from_reader(r: impl Read) -> Result<Config, Box<dyn std::error::Error>> {
        Ok(serde_json::from_reader(r)?)
    }
}

#[test]
fn test_config_from_file() {
    let cfg = Config::from_file("fanlin.json").unwrap();
    assert_eq!(cfg.port, 3000);
}

#[test]
fn test_config_from_file_not_found() {
    assert!(Config::from_file("not_found.json").is_err());
}

#[test]
fn test_legit_config() {
    let cfg = r#"
        {
          "port": 3000,
          "bind_addr": "0.0.0.0",
          "client": {
            "s3": {
              "aws_region": "ap-northeast-1",
              "aws_endpoint_url": "http://127.0.0.1:4567",
              "aws_access_key_id": "dummy_key",
              "aws_secret_access_key": "dummy_secret"
            },
            "web": {
              "user_agent": "fanlin-rs/0.0.1",
              "timeout": 5
            }
          },
          "providers": [
            {
              "path": "foo",
              "src": "s3://local-test/images"
            },
            {
              "path": "bar",
              "src": "http://127.0.0.1:3000/foo"
            }
          ]
        }
    "#;

    let got = Config::from_reader(cfg.as_bytes()).unwrap();
    assert_eq!(got.port, 3000);
    assert_eq!(got.bind_addr, "0.0.0.0");
    assert_eq!(got.client.s3.aws_region, "ap-northeast-1".to_string());
    assert_eq!(
        got.client.s3.aws_endpoint_url,
        Some("http://127.0.0.1:4567".to_string())
    );
    assert_eq!(
        got.client.s3.aws_access_key_id,
        Some("dummy_key".to_string())
    );
    assert_eq!(
        got.client.s3.aws_secret_access_key,
        Some("dummy_secret".to_string())
    );
    assert_eq!(got.providers.len(), 2);
    assert_eq!(got.providers[0].path, "foo".to_string());
    assert_eq!(got.providers[0].src, "s3://local-test/images".to_string());
    assert_eq!(got.providers[1].path, "bar".to_string());
    assert_eq!(
        got.providers[1].src,
        "http://127.0.0.1:3000/foo".to_string()
    );
}

#[test]
fn test_empty_config() {
    let cfg = "{}";
    assert!(Config::from_reader(cfg.as_bytes()).is_err());
}

#[test]
fn test_not_json_config() {
    let cfg = "---";
    assert!(Config::from_reader(cfg.as_bytes()).is_err());
}

#[test]
fn test_config_with_trailing_comma() {
    let cfg = r#"
        {
          "port": 3000,
          "bind_addr": "0.0.0.0",
          "client": {
            "s3": {
              "aws_region": "ap-northeast-1",
            },
            "web": {
              "user_agent": "fanlin-rs/0.0.1",
              "timeout": 5,
            },
          },
          "providers": [
            {
              "path": "foo",
              "src": "s3://local-test/images",
            },
            {
              "path": "bar",
              "src": "http://127.0.0.1:3000/foo",
            },
          ],
        }
    "#;

    assert!(Config::from_reader(cfg.as_bytes()).is_err());
}

#[test]
fn test_optional_config() {
    let cfg = r#"
        {
          "port": 3000,
          "bind_addr": "0.0.0.0",
          "client": {
            "s3": {
              "aws_region": "ap-northeast-1"
            },
            "web": {
              "user_agent": "fanlin-rs/0.0.1",
              "timeout": 5
            }
          },
          "providers": [
            {
              "path": "foo",
              "src": "s3://local-test/images"
            },
            {
              "path": "bar",
              "src": "http://127.0.0.1:3000/foo"
            }
          ]
        }
    "#;

    let got = Config::from_reader(cfg.as_bytes()).unwrap();
    assert_eq!(got.client.s3.aws_endpoint_url, None);
    assert_eq!(got.client.s3.aws_access_key_id, None);
    assert_eq!(got.client.s3.aws_secret_access_key, None);
}
