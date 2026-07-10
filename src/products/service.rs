use std::sync::Arc;
use uuid::Uuid;

use super::model::{CreateProductRequest, Product};
use super::repository::ProductRepository;
use crate::error::Result;

/// Service layer for Product domain - contains business logic
pub struct ProductService {
    repository: Arc<dyn ProductRepository>,
}

impl ProductService {
    pub fn new(repository: Arc<dyn ProductRepository>) -> Self {
        Self { repository }
    }

    /// Create a new product with generated ID
    pub async fn create_product(
        &self,
        tenant_id: String,
        req: CreateProductRequest,
    ) -> Result<Product> {
        // Business validation: SKU uniqueness within tenant
        if let Ok(_) = self.repository.get_by_sku(&tenant_id, &req.sku).await {
            return Err(crate::error::AppError::Conflict(format!(
                "SKU '{}' already exists for this tenant",
                req.sku
            )));
        }

        // Business validation: price and stock
        if req.price < 0.0 {
            return Err(crate::error::AppError::BadRequest(
                "price must be non-negative".into(),
            ));
        }
        if req.stock < 0 {
            return Err(crate::error::AppError::BadRequest(
                "stock must be non-negative".into(),
            ));
        }

        let product = Product {
            id: format!("prod-{}", Uuid::new_v4().simple()),
            tenant_id,
            name: req.name,
            sku: req.sku,
            price: req.price,
            stock: req.stock,
        };

        self.repository.create(product.clone()).await?;
        Ok(product)
    }

    /// Get a product by ID
    pub async fn get_product(&self, id: &str) -> Result<Product> {
        self.repository.get(id).await
    }

    /// List all products for a tenant
    pub async fn list_products_by_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<Product>> {
        self.repository.list_by_tenant(tenant_id).await
    }
}
