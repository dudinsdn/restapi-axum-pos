use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::audit::AuditLogRepository;
use crate::categories::CategoryRepository;
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{OwnerUser, UserRepository};

use super::model::{ProfitReport, ProfitReportQuery};
use super::service;

/// Profit report (revenue minus cost of goods), both total and per
/// product, with an optional time-range filter via `?from=` / `?to=`
/// (unix timestamp seconds). ONLY the owner can access this — unlike the
/// audit log which admins can still view, this report exposes the
/// store's profit margin, so it's intentionally restricted more tightly
/// via `OwnerUser`.
pub async fn profit_report<TR, PR, OR, UR, AR, CR, KR>(
    OwnerUser(auth_user): OwnerUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR, KR>>>,
    Query(query): Query<ProfitReportQuery>,
) -> Result<Json<ProfitReport>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
    KR: CategoryRepository,
{
    let report = service::profit_report(
        &state.orders,
        &state.tenants,
        &auth_user.tenant_id,
        query.from,
        query.to,
    )
    .await?;

    Ok(Json(report))
}
