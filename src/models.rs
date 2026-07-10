use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub stock: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderItem {
    pub sku: String,
    pub name: String,
    pub quantity: i32,
    pub unit_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub tenant_id: String,
    pub customer_name: String,
    pub items: Vec<OrderItem>,
    pub total: f64,
}

#[derive(Debug, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub slug: String,
    pub address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub stock: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub customer_name: String,
    pub items: Vec<CreateOrderItemRequest>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderItemRequest {
    pub sku: String,
    pub name: String,
    pub quantity: i32,
    pub unit_price: f64,
}
