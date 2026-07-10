use std::net::SocketAddr;

use restapi_axum_pos::app::create_app;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "info,axum=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = create_app();
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
