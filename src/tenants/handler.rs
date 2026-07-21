use axum::{Json, extract::State};

use crate::audit::AuditLogRepository;
use crate::categories::CategoryRepository;
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::DynState;
use crate::users::{AuthUser, UserRepository};

use super::model::Tenant;
use super::repository::TenantRepository;
use super::service;

/// Info of the tenant belonging to the currently logged-in user. There's
/// no more "list all tenants" endpoint — each user can only view their own tenant.
pub async fn get_me<TR, PR, OR, UR, AR, CR, KR>(
    auth_user: AuthUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
) -> Result<Json<Tenant>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let tenant =
        service::get_tenant(&state.tenants, &auth_user.tenant_id).await?;
    Ok(Json(tenant))
}
