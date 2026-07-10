use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;

use super::model::Product;
use crate::error::{AppError, Result};

/// Repository trait for Product persistence
#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn create(&self, product: Product) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Product>;
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<Product>>;
    async fn get_by_sku(&self, tenant_id: &str, sku: &str) -> Result<Product>;
}

/// In-memory implementation of ProductRepository
pub struct InMemoryProductRepository {
    data: RwLock<HashMap<String, Product>>,
}

impl InMemoryProductRepository {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryProductRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProductRepository for InMemoryProductRepository {
    async fn create(&self, product: Product) -> Result<()> {
        let mut data = self.data.write();
        if data.contains_key(&product.id) {
            return Err(AppError::Conflict("product already exists".into()));
        }
        data.insert(product.id.clone(), product);
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Product> {
        self.data
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("product not found".into()))
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<Product>> {
        Ok(self
            .data
            .read()
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .cloned()
            .collect())
    }

    async fn get_by_sku(&self, tenant_id: &str, sku: &str) -> Result<Product> {
        self.data
            .read()
            .values()
            .find(|p| p.tenant_id == tenant_id && p.sku == sku)
            .cloned()
            .ok_or_else(|| AppError::NotFound("product not found".into()))
    }
}
