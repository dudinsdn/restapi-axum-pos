use std::collections::HashMap;
use std::future::Future;

use parking_lot::RwLock;

use super::model::Tenant;

/// Storage contract for tenants. Methods are declared via
/// `-> impl Future<...> + Send` (instead of `async fn` sugar) so the
/// `Send` bound is guaranteed at the trait level — important because this
/// trait is used generically in Axum handlers, which require its future
/// to be `Send`.
///
/// The current in-memory implementation never actually `.await`s
/// anything (it's purely synchronous `RwLock` operations), but the
/// signature is already async from the start. So if it's swapped to
/// Postgres/SQLite later, just build a new struct implementing this
/// trait — the handler and service don't need to change at all.
pub trait TenantRepository: Send + Sync + 'static {
    fn create(&self, tenant: Tenant) -> impl Future<Output = bool> + Send;
    fn get(&self, id: &str) -> impl Future<Output = Option<Tenant>> + Send;
    fn list(&self) -> impl Future<Output = Vec<Tenant>> + Send;
    /// Used to roll back if the register process fails after the tenant
    /// was already created (e.g. the email is already in use).
    fn delete(&self, id: &str) -> impl Future<Output = ()> + Send;
}

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
