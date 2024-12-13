use crate::{log_level, Config};
use axum::{
    body::{to_bytes, Body},
    http::{HeaderMap, HeaderName, Request, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use std::{
    io,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};
use tokio::{io::AsyncReadExt, net::TcpStream};
use tokio::{
    io::AsyncWriteExt,
    net::{lookup_host, TcpListener},
};
use tracing::{error, info};
use tracing_subscriber::FmtSubscriber;
use url::Url;

pub struct Balancer {
    config: Config,
    next_server: AtomicUsize,
}

impl Balancer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            next_server: 0.into(),
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
        let listener = TcpListener::bind(listen_addr).await.unwrap();

        axum::serve(listener, app).await.unwrap();
    }

    async fn root(&self, req: Request<axum::body::Body>) -> Response {
        let next_index =
            self.next_server.fetch_add(1, Ordering::Relaxed) % self.config.servers.len();
        let server = &self.config.servers[next_index];

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

        if self.is_logging_enabled() {
            let body_text = String::from_utf8_lossy(&body_bytes);
            info!(
                "Incoming request: method={}, path={}, query={}",
                method, original_path, query
            );
            info!("Incoming request headers: {:?}", headers);
            info!("Incoming request body: {}", body_text);
            info!("Routing to backend server: {}", server.url);
            info!("Forwarding request to: {}", new_uri);
        }

        let server_addr = match resolve_url_to_ip(&server.url).await {
            Ok(addr) => addr,
            Err(e) => {
                if self.is_logging_enabled() {
                    error!("Failed to resolve server URL to IP address: {:?}", e);
                }
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap();
            }
        };

        let mut stream = match TcpStream::connect(&server_addr).await {
            Ok(s) => s,
            Err(e) => {
                if self.is_logging_enabled() {
                    error!("Failed to connect to backend server: {:?}", e);
                }
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap();
            }
        };

        let request_data = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\n{}\r\n\r\n",
            method,
            new_uri,
            server.url,
            format_headers_for_tcp(&headers)
        );

        if let Err(e) = stream.write_all(request_data.as_bytes()).await {
            if self.is_logging_enabled() {
                error!("Failed to send request to backend server: {:?}", e);
            }
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        }

        if let Err(e) = stream.write_all(&body_bytes).await {
            if self.is_logging_enabled() {
                error!("Failed to send request body to backend server: {:?}", e);
            }
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::empty())
                .unwrap();
        }

        let mut response_data = vec![0; 1024];
        match stream.read(&mut response_data).await {
            Ok(_) => {
                let response_str = String::from_utf8_lossy(&response_data);
                let (status, headers, body) = parse_tcp_response(&response_str);

                if self.is_logging_enabled() {
                    info!("Received response from backend: status={}", status);
                }

                let mut response_builder = Response::builder()
                    .status(status)
                    .body(Body::from(body))
                    .unwrap();

                *response_builder.headers_mut() = headers;
                response_builder
            }
            Err(e) => {
                if self.is_logging_enabled() {
                    error!("Failed to receive response from backend server: {:?}", e);
                }
                Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap()
            }
        }
    }
}

fn format_headers_for_tcp(headers: &HeaderMap) -> String {
    headers
        .iter()
        .map(|(key, value)| format!("{}: {}", key, value.to_str().unwrap()))
        .collect::<Vec<String>>()
        .join("\r\n")
}

fn parse_tcp_response(response: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    // Split the response into status, headers, and body.
    // This is a simplified parsing method and might need improvements.
    let mut parts = response.split("\r\n\r\n");
    let headers_part = parts.next().unwrap_or("");
    let body_part = parts.next().unwrap_or("");

    let mut headers = HeaderMap::new();
    let mut status = StatusCode::INTERNAL_SERVER_ERROR;

    for line in headers_part.lines() {
        if line.starts_with("HTTP/") {
            if let Some(status_code) = line.split_whitespace().nth(1) {
                if let Ok(code) = status_code.parse::<u16>() {
                    status = StatusCode::from_u16(code).unwrap();
                }
            }
        } else if let Some((key, value)) = line.split_once(": ") {
            headers.insert(
                HeaderName::from_bytes(key.as_bytes()).unwrap(),
                value.parse().unwrap(),
            );
        }
    }

    (status, headers, body_part.as_bytes().to_vec())
}

async fn resolve_url_to_ip(url: &str) -> Result<String, io::Error> {
    let parsed_url =
        Url::parse(url).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid URL"))?;

    let host = parsed_url
        .host_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No host found"))?
        .to_string();

    let port = parsed_url.port_or_known_default().unwrap_or(80); // Use 80 for HTTP, 443 for HTTPS

    let socket_addrs = lookup_host((host.to_string(), port)).await?;

    if let Some(addr) = socket_addrs.into_iter().next() {
        Ok(addr.to_string())
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No IP address found",
        ))
    }
}
