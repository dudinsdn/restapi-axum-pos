use std::collections::HashMap;
use std::future::Future;

use parking_lot::RwLock;

use super::model::Order;

pub trait OrderRepository: Send + Sync + 'static {
    fn create(&self, order: Order) -> impl Future<Output = bool> + Send;
    fn list_by_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Vec<Order>> + Send;
    fn get(&self, id: &str) -> impl Future<Output = Option<Order>> + Send;
    fn delete(&self, id: &str) -> impl Future<Output = bool> + Send;
}

#[derive(Debug, Default)]
pub struct InMemoryOrderRepository {
    data: RwLock<HashMap<String, Order>>,
}

impl InMemoryOrderRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OrderRepository for InMemoryOrderRepository {
    async fn create(&self, order: Order) -> bool {
        if self.data.read().contains_key(&order.id) {
            return false;
        }

        self.data.write().insert(order.id.clone(), order);
        true
    }

    async fn list_by_tenant(&self, tenant_id: &str) -> Vec<Order> {
        self.data
            .read()
            .values()
            .filter(|order| order.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    async fn get(&self, id: &str) -> Option<Order> {
        self.data.read().get(id).cloned()
    }

    async fn delete(&self, id: &str) -> bool {
        self.data.write().remove(id).is_some()
    }
}
