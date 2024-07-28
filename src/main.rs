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
        log_level = 'info'
        algorithm = 'round-robin'

        [[servers]]
        name = "api1"
        url = "https://jsonplaceholder.typicode.com/posts"
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
