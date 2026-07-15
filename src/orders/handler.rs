use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::audit::{AuditAction, AuditLogRepository, ResourceType};
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, ManagerUser, Role, UserRepository};

use super::model::{CreateOrderRequest, OrderResponse};
use super::repository::OrderRepository;
use super::service;

/// `tenant_id` always comes from the token (`AuthUser`), never from the URL —
/// same as products.
pub async fn list_orders<TR, PR, OR, UR, AR, CR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
) -> Result<Json<Vec<OrderResponse>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let orders = service::list_orders(
        &state.orders,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;

    let can_see_unit_cost = matches!(auth_user.role, Role::Owner | Role::Admin);
    let response = orders
        .into_iter()
        .map(|order| OrderResponse::from_order(order, can_see_unit_cost))
        .collect();

    Ok(Json(response))
}

pub async fn create_order<TR, PR, OR, UR, AR, CR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<OrderResponse>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
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

    let can_see_unit_cost = matches!(auth_user.role, Role::Owner | Role::Admin);
    Ok((
        StatusCode::CREATED,
        Json(OrderResponse::from_order(order, can_see_unit_cost)),
    ))
}

/// Owner and Admin can cancel an order — Cashier cannot, so they can't cover up
/// a transaction that was already made (e.g. creating an order and then
/// cancelling it themselves to hide a cash sale).
pub async fn cancel_order<TR, PR, OR, UR, AR, CR>(
    ManagerUser(auth_user): ManagerUser,
    Path(order_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
) -> Result<StatusCode>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
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
