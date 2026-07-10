use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;

use super::model::{CreateTenantRequest, Tenant};
use super::repository::TenantRepository;
use super::service;

pub async fn list_tenants<TR, PR, OR>(
    State(state): State<Arc<AppState<TR, PR, OR>>>,
) -> Result<Json<Vec<Tenant>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
{
    Ok(Json(service::list_tenants(&state.tenants).await))
}

pub async fn create_tenant<TR, PR, OR>(
    State(state): State<Arc<AppState<TR, PR, OR>>>,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Tenant>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
{
    let tenant = service::create_tenant(&state.tenants, payload).await?;
    Ok((StatusCode::CREATED, Json(tenant)))
}
