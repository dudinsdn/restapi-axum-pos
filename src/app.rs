use axum::{Router, extract::DefaultBodyLimit, http::Method, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{
        create_order, create_product, create_tenant, health_check, list_orders,
        list_products, list_tenants,
    },
    state::AppState,
};

pub fn create_app() -> Router {
    let state = AppState::new();

    Router::new()
        .route("/health", get(health_check))
        .route("/tenants", get(list_tenants).post(create_tenant))
        .route(
            "/tenants/:tenant_id/products",
            get(list_products).post(create_product),
        )
        .route(
            "/tenants/:tenant_id/orders",
            get(list_orders).post(create_order),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(tower_http::cors::Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
