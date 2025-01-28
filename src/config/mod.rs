pub mod s3;
pub mod web;

use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Provider {
    pub kind: &'static str,
    pub src: &'static str,
}

#[derive(Clone, Copy)]
pub struct Client {
    pub s3: s3::Config,
    pub web: web::Config,
}

#[derive(Clone)]
pub struct Config {
    pub port: &'static str,
    pub bind_addr: &'static str,
    pub client: Client,
    pub providers: HashMap<&'static str, Provider>,
}

impl Config {
    pub fn new() -> Self {
        Self {
            port: "3000",
            bind_addr: "0.0.0.0",
            client: Client {
                s3: s3::Config::new(),
                web: web::Config::new(),
            },
            providers: HashMap::from([
                (
                    "/foo",
                    Provider {
                        kind: "s3",
                        src: "local-test",
                    },
                ),
                (
                    "/bar",
                    Provider {
                        kind: "web",
                        src: "http://127.0.0.1:3000/foo",
                    },
                ),
            ]),
        }
    }
}
