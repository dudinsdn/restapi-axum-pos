use parking_lot::RwLock;
use std::collections::HashMap;

use super::super::model::Category;
use super::super::repository::CategoryRepository;

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
