use std::collections::HashMap;
use std::future::Future;

use parking_lot::RwLock;

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

#[derive(Debug, Default)]
pub struct InMemoryCategoryRepository {
    data: RwLock<HashMap<String, Category>>,
}

impl InMemoryCategoryRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl CategoryRepository for InMemoryCategoryRepository {
    async fn create(&self, category: Category) -> bool {
        // A single write-lock to check id + name (scoped per tenant) AND
        // insert at once, so it's atomic — same as the phone check in
        // CustomerRepository.
        let mut data = self.data.write();

        let name_taken = data.values().any(|existing| {
            existing.tenant_id == category.tenant_id
                && existing.name.eq_ignore_ascii_case(&category.name)
        });

        if name_taken || data.contains_key(&category.id) {
            return false;
        }

        data.insert(category.id.clone(), category);
        true
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Category> {
        self.data
            .read()
            .values()
            .filter(|category| category.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    async fn get(&self, id: &str) -> Option<Category> {
        self.data.read().get(id).cloned()
    }

    async fn get_by_name(
        &self,
        tenant_id: &str,
        name: &str,
    ) -> Option<Category> {
        self.data
            .read()
            .values()
            .find(|category| {
                category.tenant_id == tenant_id
                    && category.name.eq_ignore_ascii_case(name)
            })
            .cloned()
    }

    async fn update(&self, category: Category) -> bool {
        let mut data = self.data.write();
        if !data.contains_key(&category.id) {
            return false;
        }
        data.insert(category.id.clone(), category);
        true
    }

    async fn delete(&self, id: &str) -> bool {
        self.data.write().remove(id).is_some()
    }
}
