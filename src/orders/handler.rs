use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::error::{AppError, Result};
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{AuthUser, UserRepository};

use super::model::{CreateOrderRequest, Order};
use super::repository::OrderRepository;
use super::service;

fn ensure_own_tenant(auth_user: &AuthUser, tenant_id: &str) -> Result<()> {
    if auth_user.tenant_id != tenant_id {
        return Err(AppError::Forbidden(
            "not allowed to access this tenant's data".into(),
        ));
    }
    Ok(())
}

pub async fn list_orders<TR, PR, OR, UR>(
    Path(tenant_id): Path<String>,
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
) -> Result<Json<Vec<Order>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    ensure_own_tenant(&auth_user, &tenant_id)?;
    let orders =
        service::list_orders(&state.orders, &state.tenants, &tenant_id).await?;
    Ok(Json(orders))
}

pub async fn create_order<TR, PR, OR, UR>(
    Path(tenant_id): Path<String>,
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<Order>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    ensure_own_tenant(&auth_user, &tenant_id)?;
    let order = service::create_order(
        &state.orders,
        &state.products,
        &state.tenants,
        &tenant_id,
        payload,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(order)))
}
