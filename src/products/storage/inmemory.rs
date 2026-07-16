use parking_lot::RwLock;
use std::collections::HashMap;

use super::super::model::Product;
use super::super::repository::ProductRepository;

#[derive(Debug, Default)]
pub struct InMemoryProductRepository {
    data: RwLock<HashMap<String, Product>>,
}

impl InMemoryProductRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ProductRepository for InMemoryProductRepository {
    async fn create(&self, product: Product) -> bool {
        // A single write-lock to check id + sku (scoped per tenant) AND
        // insert at once, so it's atomic — same as the slug fix in tenants.
        let mut data = self.data.write();

        let sku_taken = data.values().any(|existing| {
            existing.tenant_id == product.tenant_id
                && existing.sku == product.sku
        });

        if sku_taken || data.contains_key(&product.id) {
            return false;
        }

        data.insert(product.id.clone(), product);
        true
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Product> {
        self.data
            .read()
            .values()
            .filter(|product| product.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    async fn get_by_sku(&self, tenant_id: &str, sku: &str) -> Option<Product> {
        self.data
            .read()
            .values()
            .find(|product| {
                product.tenant_id == tenant_id && product.sku == sku
            })
            .cloned()
    }

    async fn get(&self, id: &str) -> Option<Product> {
        self.data.read().get(id).cloned()
    }

    async fn update(&self, product: Product) -> bool {
        let mut data = self.data.write();
        if !data.contains_key(&product.id) {
            return false;
        }
        data.insert(product.id.clone(), product);
        true
    }

    async fn delete(&self, id: &str) -> bool {
        self.data.write().remove(id).is_some()
    }

    async fn reserve_stock(&self, product_id: &str, quantity: i32) -> bool {
        let mut data = self.data.write();
        if let Some(product) = data.get_mut(product_id) {
            if product.stock >= quantity {
                product.stock -= quantity;
                return true;
            }
        }
        false
    }

    async fn release_stock(&self, product_id: &str, quantity: i32) {
        if let Some(product) = self.data.write().get_mut(product_id) {
            product.stock += quantity;
        }
    }
}
