use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::audit::{AuditAction, AuditLogRepository, ResourceType};
use crate::error::Result;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{Actor, AuthUser, OwnerUser, UserRepository};

use super::model::{CreateOrderRequest, Order};
use super::repository::OrderRepository;
use super::service;

/// `tenant_id` selalu dari token (`AuthUser`), bukan dari URL — sama seperti
/// products.
pub async fn list_orders<TR, PR, OR, UR, AR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
) -> Result<Json<Vec<Order>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    let orders = service::list_orders(
        &state.orders,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;
    Ok(Json(orders))
}

pub async fn create_order<TR, PR, OR, UR, AR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<Order>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    let actor = Actor::from(&auth_user);
    let order = service::create_order(
        &state.orders,
        &state.products,
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

    Ok((StatusCode::CREATED, Json(order)))
}

/// Hanya owner yang boleh membatalkan order — mencegah staff/kasir menutupi
/// transaksi yang sudah dibuat (mis. buat order lalu batalkan sendiri untuk
/// menyembunyikan penjualan tunai).
pub async fn cancel_order<TR, PR, OR, UR, AR>(
    OwnerUser(auth_user): OwnerUser,
    Path(order_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
) -> Result<StatusCode>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
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
