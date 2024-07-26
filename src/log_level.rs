use serde::Deserialize;
use std::str::FromStr;
use tracing::Level;

#[derive(Debug, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    None,
    Debug,
    Error,
    Info,
    Trace,
}

impl FromStr for LogLevel {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(LogLevel::None),
            "debug" => Ok(LogLevel::Debug),
            "error" => Ok(LogLevel::Error),
            "info" => Ok(LogLevel::Info),
            "trace" => Ok(LogLevel::Trace),
            _ => Err("Invalid log level"),
        }
    }
}

impl LogLevel {
    pub fn to_tracing_level(&self) -> Level {
        match self {
            LogLevel::None => Level::DEBUG,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Error => Level::ERROR,
            LogLevel::Info => Level::INFO,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

impl ToString for LogLevel {
    fn to_string(&self) -> String {
        match self {
            LogLevel::None => "none".to_string(),
            LogLevel::Debug => "debug".to_string(),
            LogLevel::Error => "error".to_string(),
            LogLevel::Info => "info".to_string(),
            LogLevel::Trace => "trace".to_string(),
        }
    }
}
