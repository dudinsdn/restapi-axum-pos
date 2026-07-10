use std::sync::Arc;
use uuid::Uuid;

use super::model::{CreateOrderRequest, Order, OrderItem};
use super::repository::OrderRepository;
use crate::error::Result;

/// Service layer for Order domain - contains business logic
pub struct OrderService {
    repository: Arc<dyn OrderRepository>,
}

impl OrderService {
    pub fn new(repository: Arc<dyn OrderRepository>) -> Self {
        Self { repository }
    }

    /// Create a new order with calculated total
    pub async fn create_order(
        &self,
        tenant_id: String,
        req: CreateOrderRequest,
    ) -> Result<Order> {
        // Business validation: customer name not empty
        if req.customer_name.trim().is_empty() {
            return Err(crate::error::AppError::BadRequest(
                "customer_name cannot be empty".into(),
            ));
        }

        // Business validation: at least one item
        if req.items.is_empty() {
            return Err(crate::error::AppError::BadRequest(
                "order must have at least one item".into(),
            ));
        }

        // Business logic: transform and validate items
        let mut items = Vec::new();
        let mut total = 0.0;

        for item_req in req.items {
            if item_req.quantity <= 0 {
                return Err(crate::error::AppError::BadRequest(
                    "item quantity must be positive".into(),
                ));
            }
            if item_req.unit_price < 0.0 {
                return Err(crate::error::AppError::BadRequest(
                    "item price must be non-negative".into(),
                ));
            }

            let item_total = item_req.unit_price * item_req.quantity as f64;
            total += item_total;

            items.push(OrderItem {
                sku: item_req.sku,
                name: item_req.name,
                quantity: item_req.quantity,
                unit_price: item_req.unit_price,
            });
        }

        let order = Order {
            id: format!("order-{}", Uuid::new_v4().simple()),
            tenant_id,
            customer_name: req.customer_name,
            items,
            total,
        };

        self.repository.create(order.clone()).await?;
        Ok(order)
    }

    /// Get an order by ID
    pub async fn get_order(&self, id: &str) -> Result<Order> {
        self.repository.get(id).await
    }

    /// List all orders for a tenant
    pub async fn list_orders_by_tenant(
        &self,
        tenant_id: &str,
    ) -> Result<Vec<Order>> {
        self.repository.list_by_tenant(tenant_id).await
    }
}
