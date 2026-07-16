use std::future::Future;

use super::model::Category;

pub trait CategoryRepository: Send + Sync + 'static {
    fn create(&self, category: Category) -> impl Future<Output = bool> + Send;
    fn list_by_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Vec<Category>> + Send;
    /// Look up a category by its own id (across tenants) — the caller MUST
    /// check `category.tenant_id` themselves before using it, same pattern
    /// as in `ProductRepository::get`.
    fn get(&self, id: &str) -> impl Future<Output = Option<Category>> + Send;
    /// Look up a category belonging to one tenant by name. Used to check
    /// uniqueness before create/update, same pattern as
    /// `CustomerRepository::get_by_phone`.
    fn get_by_name(
        &self,
        tenant_id: &str,
        name: &str,
    ) -> impl Future<Output = Option<Category>> + Send;
    /// Overwrite an existing category. Returns `false` if the id doesn't
    /// exist at all (shouldn't happen if called after `get`).
    fn update(&self, category: Category) -> impl Future<Output = bool> + Send;
    /// Delete a category. Returns `false` if the id doesn't exist. Does
    /// NOT touch any product currently carrying this category's name as
    /// its `Product::category` — that field is a denormalized snapshot,
    /// not a foreign key (same trade-off as `Order::customer_name`
    /// surviving a deleted `Customer`), so existing products simply keep
    /// showing their old category text after it's removed from the
    /// manageable list.
    fn delete(&self, id: &str) -> impl Future<Output = bool> + Send;
}
