use crate::error::{AppError, Result};

use super::model::{CreateTenantRequest, Tenant};
use super::repository::TenantRepository;

pub async fn list_tenants<R: TenantRepository>(repo: &R) -> Vec<Tenant> {
    repo.list().await
}

pub async fn create_tenant<R: TenantRepository>(
    repo: &R,
    payload: CreateTenantRequest,
) -> Result<Tenant> {
    let tenant = Tenant {
        id: format!("tenant-{}", uuid::Uuid::new_v4().simple()),
        name: payload.name,
        slug: payload.slug,
        address: payload.address,
    };

    if !repo.create(tenant.clone()).await {
        return Err(AppError::Conflict(format!(
            "slug '{}' already in use",
            tenant.slug
        )));
    }

    Ok(tenant)
}
