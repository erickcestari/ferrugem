use serde::Deserialize;

use crate::log_level::LogLevel;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Config {
    pub port: u16,
    pub log_level: LogLevel,
    pub algorithm: String,
    pub servers: Vec<Server>,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Server {
    pub name: String,
    pub url: String,
}
