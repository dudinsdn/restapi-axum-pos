use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub sku: String,
    pub name: String,
    pub quantity: i32,
    pub unit_price: i64,
    /// Snapshot of the product's `cost_price` at the time the order was
    /// created — same reasoning as `unit_price`: if the product's
    /// `cost_price` is changed later, the profit report for OLD orders
    /// must not change along with it. Same visibility rule as
    /// `Product::cost_price`: only Owner/Admin see it in responses (see
    /// `OrderItemResponse`) — a Cashier can view an order's items and
    /// prices but never the margin behind them.
    pub unit_cost: i64,
}

/// Wire representation of an `OrderItem`. Identical to `OrderItem` except
/// `unit_cost` is optional and only populated for Owner/Admin, mirroring
/// `ProductResponse`.
#[derive(Debug, Clone, Serialize)]
pub struct OrderItemResponse {
    pub sku: String,
    pub name: String,
    pub quantity: i32,
    pub unit_price: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit_cost: Option<i64>,
}

impl OrderItemResponse {
    pub fn from_item(item: OrderItem, include_unit_cost: bool) -> Self {
        Self {
            sku: item.sku,
            name: item.name,
            quantity: item.quantity,
            unit_price: item.unit_price,
            unit_cost: include_unit_cost.then_some(item.unit_cost),
        }
    }
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
    pub total: i64,
    pub created_by: Actor,
    /// Unix timestamp (seconds) when the order was created — used by the
    /// profit report to filter a time range (`from`/`to`).
    pub created_at: u64,
}

/// Wire representation of an `Order`. Identical to `Order` except each
/// item is an `OrderItemResponse`, so `unit_cost` follows the same
/// Owner/Admin-only visibility as `ProductResponse::cost_price`.
#[derive(Debug, Clone, Serialize)]
pub struct OrderResponse {
    pub id: String,
    pub tenant_id: String,
    pub customer_id: String,
    pub customer_name: String,
    pub items: Vec<OrderItemResponse>,
    pub total: i64,
    pub created_by: Actor,
    pub created_at: u64,
}

impl OrderResponse {
    pub fn from_order(order: Order, include_unit_cost: bool) -> Self {
        Self {
            id: order.id,
            tenant_id: order.tenant_id,
            customer_id: order.customer_id,
            customer_name: order.customer_name,
            items: order
                .items
                .into_iter()
                .map(|item| {
                    OrderItemResponse::from_item(item, include_unit_cost)
                })
                .collect(),
            total: order.total,
            created_by: order.created_by,
            created_at: order.created_at,
        }
    }
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
