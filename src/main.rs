use std::net::SocketAddr;

use restapi_axum_pos::app::create_app;
use restapi_axum_pos::config::Config;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    let config = Config::from_env();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(&config.rust_log))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = create_app();
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));

    tracing::info!("Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
