use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

use crate::models::{Order, Product, Tenant};

#[derive(Debug, Default)]
pub struct AppState {
    tenants: RwLock<HashMap<String, Tenant>>,
    products: RwLock<HashMap<String, Product>>,
    orders: RwLock<HashMap<String, Order>>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn list_tenants(&self) -> Vec<Tenant> {
        self.tenants.read().values().cloned().collect()
    }

    pub fn get_tenant(&self, tenant_id: &str) -> Option<Tenant> {
        self.tenants.read().get(tenant_id).cloned()
    }

    pub fn create_tenant(&self, tenant: Tenant) -> bool {
        if self.tenants.read().contains_key(&tenant.id) {
            return false;
        }

        self.tenants.write().insert(tenant.id.clone(), tenant);
        true
    }

    pub fn list_products(&self, tenant_id: &str) -> Vec<Product> {
        self.products
            .read()
            .values()
            .filter(|product| product.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    pub fn create_product(&self, product: Product) -> bool {
        if self.products.read().contains_key(&product.id) {
            return false;
        }

        self.products.write().insert(product.id.clone(), product);
        true
    }

    pub fn list_orders(&self, tenant_id: &str) -> Vec<Order> {
        self.orders
            .read()
            .values()
            .filter(|order| order.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    pub fn create_order(&self, order: Order) -> bool {
        if self.orders.read().contains_key(&order.id) {
            return false;
        }

        self.orders.write().insert(order.id.clone(), order);
        true
    }
}
