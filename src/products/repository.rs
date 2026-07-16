use std::future::Future;

use super::model::Product;

pub trait ProductRepository: Send + Sync + 'static {
    fn create(&self, product: Product) -> impl Future<Output = bool> + Send;
    fn list_by_tenant(
        &self,
        tenant_id: &str,
    ) -> impl Future<Output = Vec<Product>> + Send;
    /// Look up a product belonging to one tenant by SKU. Used by orders to
    /// fetch the actual name & price, not from client input.
    fn get_by_sku(
        &self,
        tenant_id: &str,
        sku: &str,
    ) -> impl Future<Output = Option<Product>> + Send;
    /// Look up a product by its own id (across tenants) — the caller MUST
    /// check `product.tenant_id` themselves before using it, because this
    /// method is intentionally not scoped per tenant (used for an initial
    /// lookup before knowing who owns it).
    fn get(&self, id: &str) -> impl Future<Output = Option<Product>> + Send;
    /// Overwrite an existing product. Returns `false` if the id doesn't
    /// exist at all (shouldn't happen if called after `get`).
    fn update(&self, product: Product) -> impl Future<Output = bool> + Send;
    /// Delete a product. Returns `false` if the id doesn't exist.
    fn delete(&self, id: &str) -> impl Future<Output = bool> + Send;
    /// Reduce stock atomically. Returns `false` if the product doesn't
    /// exist or stock isn't sufficient — no change occurs in that case.
    fn reserve_stock(
        &self,
        product_id: &str,
        quantity: i32,
    ) -> impl Future<Output = bool> + Send;
    /// Return stock that was already reserved (used to roll back a failed
    /// order, or when an order is cancelled).
    fn release_stock(
        &self,
        product_id: &str,
        quantity: i32,
    ) -> impl Future<Output = ()> + Send;
}
