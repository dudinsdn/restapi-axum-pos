use std::sync::Arc;

use axum::{Json, extract::State};

use crate::audit::AuditLogRepository;
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::users::{AuthUser, UserRepository};

use super::model::Tenant;
use super::repository::TenantRepository;
use super::service;

/// Info of the tenant belonging to the currently logged-in user. There's
/// no more "list all tenants" endpoint — each user can only view their own tenant.
pub async fn get_me<TR, PR, OR, UR, AR, CR>(
    auth_user: AuthUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
) -> Result<Json<Tenant>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let tenant =
        service::get_tenant(&state.tenants, &auth_user.tenant_id).await?;
    Ok(Json(tenant))
}
