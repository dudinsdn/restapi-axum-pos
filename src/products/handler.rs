use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::error::Result;
use crate::orders::OrderRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;

use super::model::{CreateProductRequest, Product};
use super::repository::ProductRepository;
use super::service;

pub async fn list_products<TR, PR, OR>(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR>>>,
) -> Result<Json<Vec<Product>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
{
    let products =
        service::list_products(&state.products, &state.tenants, &tenant_id)
            .await?;
    Ok(Json(products))
}

pub async fn create_product<TR, PR, OR>(
    Path(tenant_id): Path<String>,
    State(state): State<Arc<AppState<TR, PR, OR>>>,
    Json(payload): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<Product>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
{
    let product = service::create_product(
        &state.products,
        &state.tenants,
        &tenant_id,
        payload,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(product)))
}
