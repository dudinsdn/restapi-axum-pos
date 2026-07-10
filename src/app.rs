use axum::{Router, extract::DefaultBodyLimit, http::Method, routing::get};
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::orders;
use crate::products;
use crate::state::AppState;
use crate::tenants;

/// Health check handler
pub async fn health_check() -> axum::http::StatusCode {
    axum::http::StatusCode::OK
}

/// Create application router with all routes
pub fn create_app() -> Router {
    let state = AppState::new();

    Router::new()
        .route("/health", get(health_check))
        .route(
            "/tenants",
            get(tenants::list_tenants)
                .post(tenants::create_tenant)
                .with_state(Arc::clone(&state.tenant_service)),
        )
        .route(
            "/tenants/:tenant_id/products",
            get(products::list_products)
                .post(products::create_product)
                .with_state(Arc::clone(&state.product_service)),
        )
        .route(
            "/tenants/:tenant_id/orders",
            get(orders::list_orders)
                .post(orders::create_order)
                .with_state(Arc::clone(&state.order_service)),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
                .allow_headers(tower_http::cors::Any),
        )
        .layer(TraceLayer::new_for_http())
}
