use axum::{Json, extract::State, http::StatusCode};
use std::sync::Arc;

use super::model::{CreateTenantRequest, Tenant};
use super::service::TenantService;
use crate::error::Result;

/// Thin handler layer - extraction + delegation
pub async fn list_tenants(
    State(service): State<Arc<TenantService>>,
) -> Result<Json<Vec<Tenant>>> {
    let tenants = service.list_tenants().await?;
    Ok(Json(tenants))
}

/// Thin handler layer - extraction + delegation
pub async fn create_tenant(
    State(service): State<Arc<TenantService>>,
    Json(payload): Json<CreateTenantRequest>,
) -> Result<(StatusCode, Json<Tenant>)> {
    let tenant = service.create_tenant(payload).await?;
    Ok((StatusCode::CREATED, Json(tenant)))
}
