use crate::{log_level, Config};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, Request},
    response::Response,
    routing::any,
    Router,
};
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

    async fn root(&self, req: Request<axum::body::Body>) -> Response<Body> {
        let mut next_server = self.next_server.lock().await;
        let server = &self.config.servers[*next_server];
        *next_server = (*next_server + 1) % self.config.servers.len();
        info!("Forwarding request to {}", server.name);

        let request = self
            .http_client
            .request(req.method().clone(), server.url.clone())
            .headers(convert_headers(req.headers()))
            .build()
            .unwrap();

        let result = self.http_client.execute(request).await;

        match result {
            Ok(response) => {
                info!("Received response with status: {}", response.status());
                let status = response.status();
                let headers = convert_headers_back(response.headers());
                let body = response.bytes().await.unwrap();

                axum::http::Response::builder()
                    .status(status)
                    .body(Body::from(body))
                    .unwrap()
            }
            Err(e) => {
                info!("Failed to send request: {:?}", e);
                let status = e
                    .status()
                    .map(|status| status.as_u16())
                    .unwrap_or_else(|| 500);

                axum::http::Response::builder()
                    .status(status)
                    .body(Body::empty())
                    .unwrap()
            }
        }
    }
}

fn convert_headers(headers: &HeaderMap) -> reqwest::header::HeaderMap {
    let mut reqwest_headers = reqwest::header::HeaderMap::new();
    for (key, value) in headers.iter() {
        reqwest_headers.insert(key, value.clone());
    }
    reqwest_headers
}

fn convert_headers_back(headers: &reqwest::header::HeaderMap) -> HeaderMap {
    let mut axum_headers = HeaderMap::new();
    for (key, value) in headers.iter() {
        axum_headers.insert(
            HeaderName::from_bytes(key.as_str().as_bytes()).unwrap(),
            value.clone(),
        );
    }
    axum_headers
}
