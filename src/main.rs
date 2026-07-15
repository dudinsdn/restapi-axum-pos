use std::net::SocketAddr;

use restapi_axum_pos::{
    app::create_app,
    audit::{InMemoryAuditLogRepository, PgAuditLogRepository},
    config::Config,
    customers::{InMemoryCustomerRepository, PgCustomerRepository},
    orders::{InMemoryOrderRepository, PgOrderRepository},
    products::{InMemoryProductRepository, PgProductRepository},
    state::AppState,
    tenants::{InMemoryTenantRepository, PgTenantRepository},
    users::{InMemoryUserRepository, PgUserRepository},
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

    // Config is read after tracing is ready, so the JWT_SECRET warning
    // (if it hasn't been set) actually shows up in the log.
    let config = Config::from_env();

    // `create_app` is generic over the repository types, so both branches
    // below monomorphize separately but still produce the same concrete
    // `Router` — that's what makes this runtime switch possible at all
    // without an enum wrapper or `dyn Trait`: nothing outside this
    // function needs to know which backend is in use.
    let app = if let Some(database_url) = &config.database_url {
        tracing::info!("Connecting to Postgres...");
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .expect("failed to connect to DATABASE_URL");

        tracing::info!("Running migrations...");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("failed to run migrations");

        let state = AppState::new(
            PgTenantRepository::new(pool.clone()),
            PgProductRepository::new(pool.clone()),
            PgOrderRepository::new(pool.clone()),
            PgUserRepository::new(pool.clone()),
            PgAuditLogRepository::new(pool.clone()),
            PgCustomerRepository::new(pool),
            config.jwt_secret.clone(),
        );
        create_app(state)
    } else {
        let state = AppState::new(
            InMemoryTenantRepository::new(),
            InMemoryProductRepository::new(),
            InMemoryOrderRepository::new(),
            InMemoryUserRepository::new(),
            InMemoryAuditLogRepository::new(),
            InMemoryCustomerRepository::new(),
            config.jwt_secret.clone(),
        );
        create_app(state)
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));

    tracing::info!("Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
