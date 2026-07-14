use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub sku: String,
    pub price: f64,
    /// Purchase price (cost of goods) — the basis for profit report
    /// calculations. Not the selling `price` customers see, so it's still
    /// shown on the regular product endpoint (anyone allowed to view
    /// products can see this), but the report that TURNS IT into a profit
    /// figure is restricted to the owner via `OwnerUser` on the
    /// `/tenants/me/reports/profit` endpoint.
    pub cost_price: f64,
    pub stock: i32,
    pub created_by: Actor,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub sku: String,
    pub price: f64,
    pub cost_price: f64,
    pub stock: i32,
}

/// Partial update (all fields optional). `sku` is intentionally NOT
/// changeable through here — sku is treated as a fixed identifier once a
/// product is created, so historical orders that store the sku as a
/// snapshot don't become ambiguous.
#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub price: Option<f64>,
    pub cost_price: Option<f64>,
    pub stock: Option<i32>,
}
