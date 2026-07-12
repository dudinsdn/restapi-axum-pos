use std::sync::Arc;

use axum::{Json, extract::State};

use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{ManagerUser, UserRepository};

use super::model::AuditLogEntry;
use super::repository::AuditLogRepository;

/// Owner dan Admin boleh melihat audit log — Cashier tidak, supaya kasir
/// tidak bisa memeriksa apakah aktivitasnya sendiri "ketahuan".
pub async fn list_audit_logs<TR, PR, OR, UR, AR>(
    ManagerUser(auth_user): ManagerUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR>>>,
) -> Result<Json<Vec<AuditLogEntry>>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
{
    let logs = state.audit.list_by_tenant(&auth_user.tenant_id).await;
    Ok(Json(logs))
}
