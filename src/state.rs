use std::sync::Arc;

use crate::orders::{InMemoryOrderRepository, OrderService};
use crate::products::{InMemoryProductRepository, ProductService};
use crate::tenants::{InMemoryTenantRepository, TenantService};

/// Application state containing all domain services
pub struct AppState {
    pub tenant_service: Arc<TenantService>,
    pub product_service: Arc<ProductService>,
    pub order_service: Arc<OrderService>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        let tenant_service = Arc::new(TenantService::new(Arc::new(
            InMemoryTenantRepository::new(),
        )));
        let product_service = Arc::new(ProductService::new(Arc::new(
            InMemoryProductRepository::new(),
        )));
        let order_service = Arc::new(OrderService::new(Arc::new(
            InMemoryOrderRepository::new(),
        )));

        Arc::new(Self {
            tenant_service,
            product_service,
            order_service,
        })
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new().as_ref().clone()
    }
}

impl Clone for AppState {
    fn clone(&self) -> Self {
        Self {
            tenant_service: Arc::clone(&self.tenant_service),
            product_service: Arc::clone(&self.product_service),
            order_service: Arc::clone(&self.order_service),
        }
    }
}
