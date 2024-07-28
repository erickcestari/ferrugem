use crate::{log_level, Config};
use axum::{http::Request, routing::any, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use tracing_subscriber::FmtSubscriber;

pub struct Balancer {
    config: Config,
    next_server: Mutex<usize>,
    http_client: reqwest::Client,
}

impl Balancer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            next_server: Mutex::new(0),
            http_client: reqwest::Client::new(),
        }
    }

    pub fn is_logging_enabled(&self) -> bool {
        self.config.log_level != log_level::LogLevel::None
    }

    pub async fn listen(self) {
        if self.is_logging_enabled() {
            let log_level = self.config.log_level.to_tracing_level();
            println!("Setting log level to {:?}", log_level);
            let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set global default subscriber");
        }

        let balancer = Arc::new(self);
        let app = {
            let balancer_clone = Arc::clone(&balancer);
            Router::new().route(
                "/",
                any(move |req| {
                    let balancer_clone = Arc::clone(&balancer_clone);
                    async move { balancer_clone.root(req).await }
                }),
            )
        };

        let listen_addr = format!("0.0.0.0:{}", balancer.config.port);
        let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();

        axum::serve(listener, app).await.unwrap();
    }

    async fn root(&self, req: Request<axum::body::Body>) -> String {
        // if self.is_logging_enabled() {
        //     info!("Request headers: {:?}", req.headers());
        //     let body_bytes = axum::body::to_bytes(req.into_body(), 0).await.unwrap();
        //     info!("Request body: {:?}", String::from_utf8_lossy(&body_bytes));
        // }

        let mut next_server = self.next_server.lock().await;
        let server = &self.config.servers[*next_server];
        *next_server = (*next_server + 1) % self.config.servers.len();
        info!("Forwarding request to {}", server.name);

        let request = self
            .http_client
            .request(req.method().clone(), server.url.clone())
            .build()
            .unwrap();
        let result = self.http_client.execute(request).await;
        let mut status = "500".to_string();

        match result {
            Ok(response) => {
                status = response.status().to_string();
                info!("Received response with status: {}", status);
            }
            Err(e) => {
                info!("Failed to send request: {:?}", e);
            }
        }

        status
    }
}
