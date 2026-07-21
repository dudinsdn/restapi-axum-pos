use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
};

use crate::audit::{AuditAction, AuditLogRepository, ResourceType};
use crate::categories::CategoryRepository;
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::pagination::{PaginationQuery, paginated_response};
use crate::products::ProductRepository;
use crate::state::DynState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, ManagerUser, Role, UserRepository};

use super::model::{CreateOrderRequest, OrderResponse};
use super::repository::OrderRepository;
use super::service;

/// `tenant_id` always comes from the token (`AuthUser`), never from the URL —
/// same as products.
///
/// Paginated via `?limit=&offset=` (see `pagination` module) — the total
/// count before slicing is returned in the `X-Total-Count` header.
pub async fn list_orders<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Response>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let orders = service::list_orders(
        &state.orders,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;

    let can_see_unit_cost = matches!(auth_user.role, Role::Owner | Role::Admin);
    let response: Vec<OrderResponse> = orders
        .into_iter()
        .map(|order| OrderResponse::from_order(order, can_see_unit_cost))
        .collect();

    Ok(paginated_response(response, &pagination))
}

/// Accepts an optional `Idempotency-Key` header. If a request with the same
/// key (scoped per tenant) already succeeded, the SAME order is returned
/// instead of creating a duplicate one — protects against a client retrying
/// after a timeout, or a cashier double-tapping "submit", from double
/// charging a customer and double-reserving stock.
///
/// Note: two truly concurrent requests with the same brand-new key can
/// still both slip through (the check-then-create isn't atomic) — see
/// `IdempotencyStore` for the tradeoffs of the current in-memory
/// implementation.
pub async fn create_order<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    headers: HeaderMap,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let idempotency_key = headers
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .filter(|key| !key.is_empty())
        .map(str::to_string);

    let can_see_unit_cost = matches!(auth_user.role, Role::Owner | Role::Admin);

    if let Some(key) = &idempotency_key {
        if let Some(existing) =
            state.idempotency_store.get(&auth_user.tenant_id, key)
        {
            return Ok((
                StatusCode::CREATED,
                Json(OrderResponse::from_order(existing, can_see_unit_cost)),
            ));
        }
    }

    let actor = Actor::from(&auth_user);
    let order = service::create_order(
        &state.orders,
        &state.products,
        &state.customers,
        &state.tenants,
        &auth_user.tenant_id,
        actor.clone(),
        payload,
    )
    .await?;

    crate::audit::service::record(
        &state.audit,
        &auth_user.tenant_id,
        &actor,
        AuditAction::Created,
        ResourceType::Order,
        &order.id,
        &format!("Order untuk {}", order.customer_name),
        Vec::new(),
    )
    .await;

    if let Some(key) = &idempotency_key {
        state
            .idempotency_store
            .put(&auth_user.tenant_id, key, order.clone());
    }

    Ok((
        StatusCode::CREATED,
        Json(OrderResponse::from_order(order, can_see_unit_cost)),
    ))
}

/// Owner and Admin can cancel an order — Cashier cannot, so they can't cover up
/// a transaction that was already made (e.g. creating an order and then
/// cancelling it themselves to hide a cash sale).
pub async fn cancel_order<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    Path(order_id): Path<String>,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
) -> Result<StatusCode>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let order = service::cancel_order(
        &state.orders,
        &state.products,
        &auth_user.tenant_id,
        &order_id,
    )
    .await?;

    crate::audit::service::record(
        &state.audit,
        &auth_user.tenant_id,
        &Actor::from(&auth_user),
        AuditAction::Deleted,
        ResourceType::Order,
        &order.id,
        &format!("Order untuk {} (dibatalkan)", order.customer_name),
        Vec::new(),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
