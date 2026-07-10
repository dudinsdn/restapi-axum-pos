use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, http::Method, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    health_check,
    orders::{self, OrderRepository},
    products::{self, ProductRepository},
    state::AppState,
    tenants::{self, TenantRepository},
};

pub fn create_app<TR, PR, OR>(state: Arc<AppState<TR, PR, OR>>) -> Router
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
{
    Router::new()
        .route("/health", get(health_check))
        .route(
            "/tenants",
            get(tenants::handler::list_tenants::<TR, PR, OR>)
                .post(tenants::handler::create_tenant::<TR, PR, OR>),
        )
        .route(
            "/tenants/:tenant_id/products",
            get(products::handler::list_products::<TR, PR, OR>)
                .post(products::handler::create_product::<TR, PR, OR>),
        )
        .route(
            "/tenants/:tenant_id/orders",
            get(orders::handler::list_orders::<TR, PR, OR>)
                .post(orders::handler::create_order::<TR, PR, OR>),
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
