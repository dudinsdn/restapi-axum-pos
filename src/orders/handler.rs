use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::error::Result;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{AuthUser, UserRepository};

use super::model::{CreateOrderRequest, Order};
use super::repository::OrderRepository;
use super::service;

/// `tenant_id` selalu dari token (`AuthUser`), bukan dari URL — sama seperti
/// products.
pub async fn list_orders<TR, PR, OR, UR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
) -> Result<Json<Vec<Order>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    let orders = service::list_orders(
        &state.orders,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;
    Ok(Json(orders))
}

pub async fn create_order<TR, PR, OR, UR>(
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
    let order = service::create_order(
        &state.orders,
        &state.products,
        &state.tenants,
        &auth_user.tenant_id,
        payload,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(order)))
}
