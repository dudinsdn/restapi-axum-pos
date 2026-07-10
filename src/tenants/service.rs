use uuid::Uuid;

use super::model::{CreateTenantRequest, Tenant};
use super::repository::TenantRepository;
use crate::error::Result;

/// Service layer for Tenant domain - contains business logic
pub struct TenantService {
    repository: std::sync::Arc<dyn TenantRepository>,
}

impl TenantService {
    pub fn new(repository: std::sync::Arc<dyn TenantRepository>) -> Self {
        Self { repository }
    }

    /// Create a new tenant with generated ID
    pub async fn create_tenant(
        &self,
        req: CreateTenantRequest,
    ) -> Result<Tenant> {
        // Business validation: slug uniqueness check
        let existing = self.repository.list().await?;
        if existing.iter().any(|t| t.slug == req.slug) {
            return Err(crate::error::AppError::Conflict(format!(
                "slug '{}' already exists",
                req.slug
            )));
        }

        let tenant = Tenant {
            id: format!("tenant-{}", Uuid::new_v4().simple()),
            name: req.name,
            slug: req.slug,
            address: req.address,
        };

        self.repository.create(tenant.clone()).await?;
        Ok(tenant)
    }

    /// Get a tenant by ID
    pub async fn get_tenant(&self, id: &str) -> Result<Tenant> {
        self.repository.get(id).await
    }

    /// List all tenants
    pub async fn list_tenants(&self) -> Result<Vec<Tenant>> {
        self.repository.list().await
    }
}
