use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

use crate::error::{AppError, Result};
use crate::orders::OrderRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{AuthUser, UserRepository};

use super::model::{CreateProductRequest, Product};
use super::repository::ProductRepository;
use super::service;

fn ensure_own_tenant(auth_user: &AuthUser, tenant_id: &str) -> Result<()> {
    if auth_user.tenant_id != tenant_id {
        return Err(AppError::Forbidden(
            "not allowed to access this tenant's data".into(),
        ));
    }
    Ok(())
}

pub async fn list_products<TR, PR, OR, UR>(
    Path(tenant_id): Path<String>,
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
) -> Result<Json<Vec<Product>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    ensure_own_tenant(&auth_user, &tenant_id)?;
    let products =
        service::list_products(&state.products, &state.tenants, &tenant_id)
            .await?;
    Ok(Json(products))
}

pub async fn create_product<TR, PR, OR, UR>(
    Path(tenant_id): Path<String>,
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
    Json(payload): Json<CreateProductRequest>,
) -> Result<(StatusCode, Json<Product>)>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    ensure_own_tenant(&auth_user, &tenant_id)?;
    let product = service::create_product(
        &state.products,
        &state.tenants,
        &tenant_id,
        payload,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(product)))
}
