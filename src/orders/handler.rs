use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::error::Result;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;

use super::model::{CreateOrderRequest, Order};
use super::repository::OrderRepository;
use super::service;

pub async fn list_orders<TR, PR, OR>(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR>>>,
) -> Result<Json<Vec<Order>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
{
    let orders =
        service::list_orders(&state.orders, &state.tenants, &tenant_id).await?;
    Ok(Json(orders))
}

pub async fn create_order<TR, PR, OR>(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR>>>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<Order>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
{
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
