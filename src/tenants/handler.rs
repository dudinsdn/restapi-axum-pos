use std::sync::Arc;

use axum::{Json, extract::State};

use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::users::{AuthUser, UserRepository};

use super::model::Tenant;
use super::repository::TenantRepository;
use super::service;

/// Info tenant milik user yang sedang login. Tidak ada lagi endpoint
/// "list semua tenant" — setiap user cuma boleh lihat tenant-nya sendiri.
pub async fn get_me<TR, PR, OR, UR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR>>>,
) -> Result<Json<Tenant>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
{
    let tenant =
        service::get_tenant(&state.tenants, &auth_user.tenant_id).await?;
    Ok(Json(tenant))
}
