use crate::error::{AppError, Result};

use super::model::Tenant;
use super::repository::TenantRepository;

pub async fn get_tenant<R: TenantRepository>(
    repo: &R,
    tenant_id: &str,
) -> Result<Tenant> {
    repo.get(tenant_id)
        .await
        .ok_or_else(|| AppError::NotFound("tenant not found".into()))
}
