use parking_lot::RwLock;
use std::collections::HashMap;

use super::super::model::Tenant;
use super::super::repository::TenantRepository;

#[derive(Debug, Default)]
pub struct InMemoryTenantRepository {
    data: RwLock<HashMap<String, Tenant>>,
}

impl InMemoryTenantRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl TenantRepository for InMemoryTenantRepository {
    async fn create(&self, tenant: Tenant) -> bool {
        // A single write-lock to check id + slug AND insert at once.
        // Intentionally not split into a separate read-check then
        // write-insert, because that opens a race condition: two
        // concurrent requests could both pass the "doesn't exist yet"
        // check before either has a chance to insert.
        let mut data = self.data.write();

        let slug_taken =
            data.values().any(|existing| existing.slug == tenant.slug);
        if slug_taken || data.contains_key(&tenant.id) {
            return false;
        }

        data.insert(tenant.id.clone(), tenant);
        true
    }

    async fn get(&self, id: &str) -> Option<Tenant> {
        self.data.read().get(id).cloned()
    }

    async fn list(&self) -> Vec<Tenant> {
        self.data.read().values().cloned().collect()
    }

    async fn delete(&self, id: &str) {
        self.data.write().remove(id);
    }
}
