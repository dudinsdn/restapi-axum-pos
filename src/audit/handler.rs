use axum::{
    extract::{Query, State},
    response::Response,
};

use crate::categories::CategoryRepository;
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::pagination::{PaginationQuery, paginated_response};
use crate::products::ProductRepository;
use crate::state::DynState;
use crate::tenants::TenantRepository;
use crate::users::{ManagerUser, UserRepository};

use super::repository::AuditLogRepository;

/// Owner and Admin can view the audit log — Cashier cannot, so a cashier
/// can't check whether their own activity has been "caught".
///
/// Paginated via `?limit=&offset=` (see `pagination` module) — the total
/// count before slicing is returned in the `X-Total-Count` header. Worth
/// using here in particular: this log is append-only and never trimmed, so
/// it only grows over the tenant's lifetime.
pub async fn list_audit_logs<TR, PR, OR, UR, AR, CR, KR>(
    ManagerUser(auth_user): ManagerUser,
    State(state): State<DynState<TR, PR, OR, UR, AR, CR, KR>>,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Response>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let logs = state.audit.list_by_tenant(&auth_user.tenant_id).await;
    Ok(paginated_response(logs, &pagination))
}
