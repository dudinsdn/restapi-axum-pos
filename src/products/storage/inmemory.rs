use parking_lot::RwLock;

use super::super::model::Product;
use super::super::repository::ProductRepository;

#[derive(Debug, Default)]
pub struct InMemoryProductRepository {
    // A Vec (rather than a HashMap) so iteration order matches insertion
    // order — callers like `list_by_tenant` rely on that for stable,
    // deterministic pagination.
    data: RwLock<Vec<Product>>,
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

        let sku_taken = data.iter().any(|existing| {
            existing.tenant_id == product.tenant_id
                && existing.sku == product.sku
        });

        if sku_taken || data.iter().any(|existing| existing.id == product.id) {
            return false;
        }

        data.push(product);
        true
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Product> {
        self.data
            .read()
            .iter()
            .filter(|product| product.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    async fn get_by_sku(&self, tenant_id: &str, sku: &str) -> Option<Product> {
        self.data
            .read()
            .iter()
            .find(|product| {
                product.tenant_id == tenant_id && product.sku == sku
            })
            .cloned()
    }

    async fn get(&self, id: &str) -> Option<Product> {
        self.data
            .read()
            .iter()
            .find(|product| product.id == id)
            .cloned()
    }

    async fn update(&self, product: Product) -> bool {
        let mut data = self.data.write();
        if let Some(existing) =
            data.iter_mut().find(|existing| existing.id == product.id)
        {
            *existing = product;
            true
        } else {
            false
        }
    }

    async fn delete(&self, id: &str) -> bool {
        let mut data = self.data.write();
        if let Some(index) = data.iter().position(|product| product.id == id) {
            data.remove(index);
            true
        } else {
            false
        }
    }

    async fn reserve_stock(&self, product_id: &str, quantity: i32) -> bool {
        let mut data = self.data.write();
        if let Some(product) =
            data.iter_mut().find(|product| product.id == product_id)
        {
            if product.stock >= quantity {
                product.stock -= quantity;
                return true;
            }
        }
        false
    }

    async fn release_stock(&self, product_id: &str, quantity: i32) {
        if let Some(product) = self
            .data
            .write()
            .iter_mut()
            .find(|product| product.id == product_id)
        {
            product.stock += quantity;
        }
    }
}
