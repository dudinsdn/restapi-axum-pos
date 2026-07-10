use async_trait::async_trait;
use parking_lot::RwLock;
use std::collections::HashMap;

use super::model::Order;
use crate::error::{AppError, Result};

/// Repository trait for Order persistence
#[async_trait]
pub trait OrderRepository: Send + Sync {
    async fn create(&self, order: Order) -> Result<()>;
    async fn get(&self, id: &str) -> Result<Order>;
    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<Order>>;
}

/// In-memory implementation of OrderRepository
pub struct InMemoryOrderRepository {
    data: RwLock<HashMap<String, Order>>,
}

impl InMemoryOrderRepository {
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryOrderRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OrderRepository for InMemoryOrderRepository {
    async fn create(&self, order: Order) -> Result<()> {
        let mut data = self.data.write();
        if data.contains_key(&order.id) {
            return Err(AppError::Conflict("order already exists".into()));
        }
        data.insert(order.id.clone(), order);
        Ok(())
    }

    async fn get(&self, id: &str) -> Result<Order> {
        self.data
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::NotFound("order not found".into()))
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Result<Vec<Order>> {
        Ok(self
            .data
            .read()
            .values()
            .filter(|o| o.tenant_id == tenant_id)
            .cloned()
            .collect())
    }
}
