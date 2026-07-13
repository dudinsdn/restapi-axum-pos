use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::audit::AuditLogRepository;
use crate::customers::CustomerRepository;
use crate::error::Result;
use crate::orders::OrderRepository;
use crate::products::ProductRepository;
use crate::state::AppState;
use crate::tenants::TenantRepository;
use crate::users::{OwnerUser, UserRepository};

use super::model::{ProfitReport, ProfitReportQuery};
use super::service;

/// Laporan profit (pendapatan dikurangi harga beli/HPP), total maupun per
/// produk, dengan filter rentang waktu opsional lewat `?from=` / `?to=`
/// (unix timestamp detik). HANYA owner yang boleh akses — beda dengan
/// audit log yang masih boleh dilihat admin, laporan ini membuka margin
/// keuntungan toko, jadi sengaja dibatasi lebih ketat lewat `OwnerUser`.
pub async fn profit_report<TR, PR, OR, UR, AR, CR>(
    OwnerUser(auth_user): OwnerUser,
    State(state): State<Arc<AppState<TR, PR, OR, UR, AR, CR>>>,
    Query(query): Query<ProfitReportQuery>,
) -> Result<Json<ProfitReport>>
where
    TR: TenantRepository,
    PR: ProductRepository,
    OR: OrderRepository,
    UR: UserRepository,
    AR: AuditLogRepository,
    CR: CustomerRepository,
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
