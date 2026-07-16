use std::future::Future;

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
