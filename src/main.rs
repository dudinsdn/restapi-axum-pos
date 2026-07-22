use std::net::SocketAddr;

use restapi_axum_pos::{
    app::create_app,
    audit::PgAuditLogRepository,
    categories::PgCategoryRepository,
    config::Config,
    customers::PgCustomerRepository,
    orders::PgOrderRepository,
    products::PgProductRepository,
    state::{AppState, Repositories},
    tenants::PgTenantRepository,
    users::PgUserRepository,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

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

    tracing::info!("Connecting to Postgres...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .expect("failed to connect to DATABASE_URL");

    tracing::info!("Running migrations...");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let repos = Repositories {
        tenants: PgTenantRepository::new(pool.clone()),
        products: PgProductRepository::new(pool.clone()),
        orders: PgOrderRepository::new(pool.clone()),
        users: PgUserRepository::new(pool.clone()),
        audit: PgAuditLogRepository::new(pool.clone()),
        customers: PgCustomerRepository::new(pool.clone()),
        categories: PgCategoryRepository::new(pool),
    };

    let state = AppState::new(repos, config.jwt_secret.clone());
    let app = create_app(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));

    tracing::info!("Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
