use std::sync::Arc;

use axum::{Json, extract::State};

use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{OwnerUser, UserRepository};

use super::model::AuditLogEntry;
use super::repository::AuditLogRepository;

/// Hanya owner yang boleh melihat audit log — ini adalah jejak siapa
/// mengubah/menghapus apa, jadi staff/kasir tidak diberi akses (mis. supaya
/// staff tidak bisa memeriksa apakah aktivitasnya sendiri "ketahuan").
pub async fn list_audit_logs<TR, PR, OR, UR, AR>(
    OwnerUser(auth_user): OwnerUser,
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
