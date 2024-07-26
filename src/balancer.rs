use axum::{http::Request, routing::any, Router};
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::Config;

#[tokio::main]
pub async fn listen(config: Config) {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");

    let app = Router::new().route("/", any(root));

    let listen_addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

async fn root(req: Request<axum::body::Body>) -> &'static str {
    info!("Request headers: {:?}", req.headers());

    let body_bytes = axum::body::to_bytes(req.into_body(), 0).await.unwrap();
    info!("Request body: {:?}", String::from_utf8_lossy(&body_bytes));

    "Hello, World!"
}
