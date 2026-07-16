use std::future::Future;

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
