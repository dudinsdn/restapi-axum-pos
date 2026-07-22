use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, http::Method, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    audit::{self, AuditLogRepository},
    categories::{self, CategoryRepository},
    customers::{self, CustomerRepository},
    health_check,
    orders::{self, OrderRepository},
    products::{self, ProductRepository},
    reports,
    state::AppState,
    tenants::{self, TenantRepository},
    users::{self, UserRepository},
};

pub fn create_app<TR, PR, OR, UR, AR, CR, KR>(
    state: Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>,
) -> Router
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    Router::new()
        .route("/health", get(health_check))
        .route(
            "/auth/register",
            axum::routing::post(users::handler::register::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/auth/login",
            axum::routing::post(users::handler::login::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/auth/logout",
            axum::routing::post(users::handler::logout::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/tenants/me",
            get(tenants::handler::get_me::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/tenants/me/users",
            axum::routing::post(
                users::handler::invite_staff::<TR, PR, OR, UR, AR, CR, KR>,
            ),
        )
        .route(
            "/tenants/me/audit-logs",
            get(audit::handler::list_audit_logs::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/tenants/me/reports/profit",
            get(reports::handler::profit_report::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/products",
            get(products::handler::list_products::<TR, PR, OR, UR, AR, CR, KR>)
                .post(products::handler::create_product::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/products/low-stock",
            get(
                products::handler::list_low_stock_products::<
                    TR,
                    PR,
                    OR,
                    UR,
                    AR,
                    CR,
                    KR,
                >,
            ),
        )
        .route(
            "/products/{product_id}",
            axum::routing::patch(
                products::handler::update_product::<TR, PR, OR, UR, AR, CR, KR>,
            )
            .delete(products::handler::delete_product::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/orders",
            get(orders::handler::list_orders::<TR, PR, OR, UR, AR, CR, KR>)
                .post(orders::handler::create_order::<TR, PR, OR, UR, AR, CR, KR>),
        )
        .route(
            "/orders/{order_id}",
            axum::routing::delete(
                orders::handler::cancel_order::<TR, PR, OR, UR, AR, CR, KR>,
            ),
        )
        .route(
            "/customers",
            get(customers::handler::list_customers::<TR, PR, OR, UR, AR, CR, KR>)
                .post(
                    customers::handler::create_customer::<
                        TR,
                        PR,
                        OR,
                        UR,
                        AR,
                        CR,
                        KR,
                    >,
                ),
        )
        .route(
            "/customers/{customer_id}",
            get(customers::handler::get_customer::<TR, PR, OR, UR, AR, CR, KR>)
                .patch(
                    customers::handler::update_customer::<
                        TR,
                        PR,
                        OR,
                        UR,
                        AR,
                        CR,
                        KR,
                    >,
                )
                .delete(
                    customers::handler::delete_customer::<
                        TR,
                        PR,
                        OR,
                        UR,
                        AR,
                        CR,
                        KR,
                    >,
                ),
        )
        .route(
            "/categories",
            get(categories::handler::list_categories::<
                TR,
                PR,
                OR,
                UR,
                AR,
                CR,
                KR,
            >)
            .post(categories::handler::create_category::<
                TR,
                PR,
                OR,
                UR,
                AR,
                CR,
                KR,
            >),
        )
        .route(
            "/categories/{category_id}",
            get(categories::handler::get_category::<TR, PR, OR, UR, AR, CR, KR>)
                .patch(
                    categories::handler::update_category::<
                        TR,
                        PR,
                        OR,
                        UR,
                        AR,
                        CR,
                        KR,
                    >,
                )
                .delete(
                    categories::handler::delete_category::<
                        TR,
                        PR,
                        OR,
                        UR,
                        AR,
                        CR,
                        KR,
                    >,
                ),
        )
        .route(
            "/categories/{category_id}/products",
            get(categories::handler::list_products_in_category::<
                TR,
                PR,
                OR,
                UR,
                AR,
                CR,
                KR,
            >),
        )
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers(tower_http::cors::Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
