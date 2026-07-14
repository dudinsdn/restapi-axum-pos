use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub sku: String,
    pub name: String,
    pub quantity: i32,
    pub unit_price: f64,
    /// Snapshot of the product's `cost_price` at the time the order was
    /// created — same reasoning as `unit_price`: if the product's
    /// `cost_price` is changed later, the profit report for OLD orders
    /// must not change along with it.
    pub unit_cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub tenant_id: String,
    /// Reference to an already-registered `Customer` — orders can NO LONGER
    /// be created with a free-form customer name; it must be an existing
    /// customer in `/customers`.
    pub customer_id: String,
    /// Snapshot of the customer's name at the time the order was created —
    /// same as the product name/price in `OrderItem`, so the order can still
    /// be displayed correctly even if the customer's name is changed later
    /// or the data is deleted.
    pub customer_name: String,
    pub items: Vec<OrderItem>,
    pub total: f64,
    pub created_by: Actor,
    /// Unix timestamp (seconds) when the order was created — used by the
    /// profit report to filter a time range (`from`/`to`).
    pub created_at: u64,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderItemRequest {
    pub sku: String,
    pub quantity: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    /// Id of an already-registered customer (see `POST /customers`).
    pub customer_id: String,
    pub items: Vec<CreateOrderItemRequest>,
}
