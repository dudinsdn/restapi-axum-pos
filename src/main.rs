use std::net::SocketAddr;

use restapi_axum_pos::{
    app::create_app, config::Config, orders::InMemoryOrderRepository,
    products::InMemoryProductRepository, state::AppState,
    tenants::InMemoryTenantRepository, users::InMemoryUserRepository,
};
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

    // Config dibaca setelah tracing siap, supaya warning JWT_SECRET
    // (kalau belum di-set) benar-benar muncul di log.
    let config = Config::from_env();

    let state = AppState::new(
        InMemoryTenantRepository::new(),
        InMemoryProductRepository::new(),
        InMemoryOrderRepository::new(),
        InMemoryUserRepository::new(),
        config.jwt_secret.clone(),
    );

    let app = create_app(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));

    tracing::info!("Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
