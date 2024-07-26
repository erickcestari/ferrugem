use crate::{log_level, Config};
use axum::{http::Request, routing::any, Router};
use tracing::info;
use tracing_subscriber::FmtSubscriber;

pub struct Balancer {
    config: Config,
}

impl Balancer {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn listen(self) {
        if self.config.log_level != log_level::LogLevel::None {
            let log_level = self.config.log_level.to_tracing_level();
            println!("Setting log level to {:?}", log_level);
            let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
            tracing::subscriber::set_global_default(subscriber)
                .expect("Failed to set global default subscriber");
        }

        let app = Router::new().route("/", any(Self::root));

        let listen_addr = format!("0.0.0.0:{}", self.config.port);
        let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();

        axum::serve(listener, app).await.unwrap();
    }

    async fn root(req: Request<axum::body::Body>) -> &'static str {
        info!("Request headers: {:?}", req.headers());

        let body_bytes = axum::body::to_bytes(req.into_body(), 0).await.unwrap();
        info!("Request body: {:?}", String::from_utf8_lossy(&body_bytes));

        "Hello, World!"
    }
}
