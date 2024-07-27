use balancer::Balancer;
use config::Config;
use toml;

mod balancer;
mod config;
mod log_level;

#[tokio::main]
async fn main() {
    let toml_str = r#"
        version = 1
        port = 9999
        log_level = 'none'
        algorithm = 'round-robin'

        [[servers]]
        name = "api1"
        address = "http://localhost:3000"

        [[servers]]
        name = "api2"
        address = "http://localhost:3001"

        [[servers]]
        name = "api3"
        address = "http://localhost:3002"
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
