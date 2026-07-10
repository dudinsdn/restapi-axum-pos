use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;

use super::model::Tenant;
use crate::error::{AppError, Result};

/// Repository trait for Tenant persistence
#[async_trait]
pub trait TenantRepository: Send + Sync {
    async fn create(&self, tenant: Tenant) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Tenant>;
    async fn list(&self) -> Result<Vec<Tenant>>;
}

/// In-memory implementation of TenantRepository
pub struct InMemoryTenantRepository {
    data: RwLock<HashMap<String, Tenant>>,
}

impl InMemoryTenantRepository {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryTenantRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TenantRepository for InMemoryTenantRepository {
    async fn create(&self, tenant: Tenant) -> Result<()> {
        let mut data = self.data.write();
        if data.contains_key(&tenant.id) {
            return Err(AppError::Conflict("tenant already exists".into()));
        }
        data.insert(tenant.id.clone(), tenant);
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Tenant> {
        self.data
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("tenant not found".into()))
    }

    async fn list(&self) -> Result<Vec<Tenant>> {
        Ok(self.data.read().values().cloned().collect())
    }
}
