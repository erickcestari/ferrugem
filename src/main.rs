use balancer::Balancer;
use config::Config;
use std::fs;
use toml;

mod balancer;
mod config;
mod log_level;

#[tokio::main]
async fn main() {
    let config_path = "ferrugem.toml";

    let toml_str = match fs::read_to_string(config_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Failed to read configuration file: {}", e);
            return;
        }
    };

    match toml::de::from_str::<Config>(&toml_str) {
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
