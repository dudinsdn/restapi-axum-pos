use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use std::sync::Arc;

use super::model::{CreateOrderRequest, Order};
use super::service::OrderService;
use crate::error::Result;

/// Thin handler layer - extraction + delegation
pub async fn list_orders(
    Path(tenant_id): Path<String>,
    State(service): State<Arc<OrderService>>,
) -> Result<Json<Vec<Order>>> {
    let orders = service.list_orders_by_tenant(&tenant_id).await?;
    Ok(Json(orders))
}

/// Thin handler layer - extraction + delegation
pub async fn create_order(
    Path(tenant_id): Path<String>,
    State(service): State<Arc<OrderService>>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<(StatusCode, Json<Order>)> {
    let order = service.create_order(tenant_id, payload).await?;
    Ok((StatusCode::CREATED, Json(order)))
}
