use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};

use crate::error::Result;
use crate::orders::OrderRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{AuthUser, UserRepository};

use super::model::{CreateProductRequest, Product};
use super::repository::ProductRepository;
use super::service;

/// `tenant_id` TIDAK diambil dari path/URL — selalu dari token yang sudah
/// terverifikasi (`AuthUser`). Jadi tidak ada "tenant_id yang salah" untuk
/// dicoba, karena client tidak pernah diminta mengirimkannya.
pub async fn list_products<TR, PR, OR, UR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
) -> Result<Json<Vec<Product>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    let products = service::list_products(
        &state.products,
        &state.tenants,
        &auth_user.tenant_id,
    )
    .await?;
    Ok(Json(products))
}

pub async fn create_product<TR, PR, OR, UR>(
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
    let product = service::create_product(
        &state.products,
        &state.tenants,
        &auth_user.tenant_id,
        payload,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(product)))
}
