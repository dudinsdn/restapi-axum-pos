use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Product {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub stock: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub stock: i32,
}
