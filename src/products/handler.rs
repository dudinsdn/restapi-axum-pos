use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use std::sync::Arc;

use super::model::{CreateProductRequest, Product};
use super::service::ProductService;
use crate::error::Result;

/// Thin handler layer - extraction + delegation
pub async fn list_products(
    Path(tenant_id): Path<String>,
    State(service): State<Arc<ProductService>>,
) -> Result<Json<Vec<Product>>> {
    let products = service.list_products_by_tenant(&tenant_id).await?;
    Ok(Json(products))
}

/// Thin handler layer - extraction + delegation
pub async fn create_product(
    Path(tenant_id): Path<String>,
    State(service): State<Arc<ProductService>>,
    Json(payload): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<Product>)> {
    let product = service.create_product(tenant_id, payload).await?;
    Ok((StatusCode::CREATED, Json(product)))
}
