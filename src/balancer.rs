use crate::{log_level, Config};
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, HeaderName, Request, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use std::{sync::Arc, usize};
use tokio::sync::Mutex;
use tracing::{error, info};
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
            Router::new()
                .route(
                    "/",
                    any({
                        let balancer_clone = Arc::clone(&balancer_clone);
                        move |req| {
                            let balancer_clone = Arc::clone(&balancer_clone);
                            async move { balancer_clone.root(req).await }
                        }
                    }),
                )
                .route(
                    "/*path",
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

    async fn root(&self, req: Request<axum::body::Body>) -> Response {
        let mut next_server = self.next_server.lock().await;
        let server = &self.config.servers[*next_server];
        *next_server = (*next_server + 1) % self.config.servers.len();

        let original_path = req.uri().path().to_string();
        let query = req
            .uri()
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default();
        let method = req.method().clone();
        let headers = req.headers().clone();
        let new_uri = format!("{}{}{}", server.url, original_path, query);
        let body_bytes = to_bytes(req.into_body(), usize::MAX).await.unwrap();
        let body_text = String::from_utf8_lossy(&body_bytes);

        if self.is_logging_enabled() {
            info!(
                "Incoming request: method={}, path={}, query={}",
                method, original_path, query
            );
            info!("Incoming request headers: {:?}", headers);
            info!("Incoming request body: {}", body_text);
            info!("Routing to backend server: {}", server.url);
            info!("Forwarding request to: {}", new_uri);
        }

        let request = self
            .http_client
            .request(method, new_uri)
            .headers(convert_headers(&headers))
            .body(body_bytes)
            .build()
            .unwrap();

        match self.http_client.execute(request).await {
            Ok(response) => {
                if self.is_logging_enabled() {
                    info!(
                        "Received response from backend server with status: {}",
                        response.status()
                    );
                    info!("Response headers: {:?}", response.headers());
                }
                let status = response.status();
                let headers = convert_headers_back(response.headers());
                let body_stream = response.bytes_stream();

                let mut response_builder = axum::response::Response::builder()
                    .status(status)
                    .body(Body::from_stream(body_stream))
                    .unwrap();

                *response_builder.headers_mut() = headers;

                response_builder
            }
            Err(e) => {
                if self.is_logging_enabled() {
                    error!("Failed to send request to backend server: {:?}", e);
                }
                let status = e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

                Response::builder()
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
    reqwest_headers.remove("host");
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
    axum_headers.insert(
        HeaderName::from_bytes("X-Powered-By".as_bytes()).unwrap(),
        "ferrugem".parse().unwrap(),
    );
    axum_headers
}
