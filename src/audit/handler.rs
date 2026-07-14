use std::sync::Arc;

use axum::{Json, extract::State};

use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{ManagerUser, UserRepository};

use super::model::AuditLogEntry;
use super::repository::AuditLogRepository;

/// Owner and Admin can view the audit log — Cashier cannot, so a cashier
/// can't check whether their own activity has been "caught".
pub async fn list_audit_logs<TR, PR, OR, UR, AR, CR>(
    ManagerUser(auth_user): ManagerUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
) -> Result<Json<Vec<AuditLogEntry>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
{
    let logs = state.audit.list_by_tenant(&auth_user.tenant_id).await;
    Ok(Json(logs))
}
