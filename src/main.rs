use balancer::Balancer;
use log_level::LogLevel;
use serde::Deserialize;
use toml;

mod balancer;
mod log_level;

#[derive(Deserialize, Debug, PartialEq)]
struct Config {
    version: u32,
    port: u16,
    log_level: LogLevel,
    algorithm: String,
    servers: Vec<Server>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Server {
    name: String,
    address: String,
}

#[tokio::main]
async fn main() {
    let toml_str = r#"
        version = 1
        port = 3333
        log_level = 'trace'
        algorithm = 'round-robin'

        [[servers]]
        name = "api1"
        address = "http://localhost:3000"

        [[servers]]
        name = "api2"
        address = "http://localhost:3001"
    "#;

    match toml::from_str::<Config>(toml_str) {
        Ok(config) => {
            println!("{:#?}", config);
            let balancer = Balancer::new(config);
            balancer.listen().await;
        }
        Err(e) => {
            eprintln!("Failed to parse config: {}", e);
        }
    }
}

#[test]
fn test_config_parsing() {
    let toml_str = r#"
            version = 1
            port = 3333
            log_level = 'none'
            algorithm = 'round-robin'

            [[servers]]
            name = "api1"
            address = "http://localhost:3000"

            [[servers]]
            name = "apicanhavediferentnamesbytheuser"
            address = "http://localhost:3001"
        "#;

    let expected_config = Config {
        version: 1,
        log_level: LogLevel::None,
        port: 3333,
        algorithm: "round-robin".to_string(),
        servers: vec![
            Server {
                name: "api1".to_string(),
                address: "http://localhost:3000".to_string(),
            },
            Server {
                name: "apicanhavediferentnamesbytheuser".to_string(),
                address: "http://localhost:3001".to_string(),
            },
        ],
    };

    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config, expected_config);
}
