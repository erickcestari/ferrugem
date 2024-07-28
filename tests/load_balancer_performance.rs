use ferrugem::config::Server;
use ferrugem::{log_level, Balancer, Config};
use reqwest::Client;
use std::time::Instant;
use tokio::task;
use tokio::time::{sleep, Duration};
use tracing::{info, subscriber::set_default};
use tracing_subscriber::fmt::Subscriber;

#[tokio::test]
async fn test_load_balancer_performance() {
    let subscriber = Subscriber::builder().finish();
    let _default = set_default(subscriber);

    let config = Config {
        port: 9999,
        log_level: log_level::LogLevel::Info,
        algorithm: "round-robin".to_string(),
        servers: vec![
            Server {
                url: "https://jsonplaceholder.typicode.com".to_string(),
                name: "api1".to_string(),
            },
            Server {
                url: "https://jsonplaceholder.typicode.com".to_string(),
                name: "api2".to_string(),
            },
        ],
    };

    let balancer = Balancer::new(config);
    let balancer_handle = task::spawn(async move {
        balancer.listen().await;
    });

    sleep(Duration::from_secs(1)).await;

    let client = Client::new();
    let mut handles = vec![];
    let mut times = vec![];

    for _ in 0..100 {
        let client = client.clone();
        handles.push(task::spawn(async move {
            let start = Instant::now();
            let res = client.get("http://127.0.0.1:9999").send().await.unwrap();
            let duration = start.elapsed();
            res.text().await.unwrap();
            duration
        }));
    }

    for handle in handles {
        let time = handle.await.unwrap();
        times.push(time);
    }

    let min_time = times.iter().min().unwrap();
    let max_time = times.iter().max().unwrap();
    let avg_time: Duration = times.iter().sum::<Duration>() / times.len() as u32;

    info!("Min time: {:?}", min_time);
    info!("Avg time: {:?}", avg_time);
    info!("Max time: {:?}", max_time);

    balancer_handle.abort();
}
