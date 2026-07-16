use std::future::Future;

use super::model::Customer;

pub trait CustomerRepository: Send + Sync + 'static {
    fn create(&self, customer: Customer) -> impl Future<Output = bool> + Send;
    fn list_by_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Vec<Customer>> + Send;
    /// Look up a customer by its own id (across tenants) — the caller MUST
    /// check `customer.tenant_id` themselves before using it, same pattern
    /// as in `ProductRepository::get`.
    fn get(&self, id: &str) -> impl Future<Output = Option<Customer>> + Send;
    /// Look up a customer belonging to one tenant by phone number. Used to
    /// check uniqueness before create/update, similar to `ProductRepository::get_by_sku`.
    fn get_by_phone(
        &self,
        tenant_id: &str,
        phone: &str,
    ) -> impl Future<Output = Option<Customer>> + Send;
    /// Overwrite an existing customer. Returns `false` if the id doesn't
    /// exist at all (shouldn't happen if called after `get`).
    fn update(&self, customer: Customer) -> impl Future<Output = bool> + Send;
    /// Delete a customer. Returns `false` if the id doesn't exist.
    fn delete(&self, id: &str) -> impl Future<Output = bool> + Send;
}
