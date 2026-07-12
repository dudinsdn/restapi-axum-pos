use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, http::Method, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    audit::{self, AuditLogRepository},
    health_check,
    orders::{self, OrderRepository},
    products::{self, ProductRepository},
    state::AppState,
    tenants::{self, TenantRepository},
    users::{self, UserRepository},
};

pub fn create_app<TR, PR, OR, UR, AR>(
    state: Arc<AppState<TR, PR, OR, UR, AR>>,
) -> Router
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    Router::new()
        .route("/health", get(health_check))
        .route(
            "/auth/register",
            axum::routing::post(users::handler::register::<TR, PR, OR, UR, AR>),
        )
        .route(
            "/auth/login",
            axum::routing::post(users::handler::login::<TR, PR, OR, UR, AR>),
        )
        .route(
            "/auth/logout",
            axum::routing::post(users::handler::logout::<TR, PR, OR, UR, AR>),
        )
        .route(
            "/tenants/me",
            get(tenants::handler::get_me::<TR, PR, OR, UR, AR>),
        )
        .route(
            "/tenants/me/users",
            axum::routing::post(
                users::handler::invite_staff::<TR, PR, OR, UR, AR>,
            ),
        )
        .route(
            "/tenants/me/audit-logs",
            get(audit::handler::list_audit_logs::<TR, PR, OR, UR, AR>),
        )
        .route(
            "/products",
            get(products::handler::list_products::<TR, PR, OR, UR, AR>)
                .post(products::handler::create_product::<TR, PR, OR, UR, AR>),
        )
        .route(
            "/products/:product_id",
            axum::routing::patch(
                products::handler::update_product::<TR, PR, OR, UR, AR>,
            )
            .delete(products::handler::delete_product::<TR, PR, OR, UR, AR>),
        )
        .route(
            "/orders",
            get(orders::handler::list_orders::<TR, PR, OR, UR, AR>)
                .post(orders::handler::create_order::<TR, PR, OR, UR, AR>),
        )
        .route(
            "/orders/:order_id",
            axum::routing::delete(
                orders::handler::cancel_order::<TR, PR, OR, UR, AR>,
            ),
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
