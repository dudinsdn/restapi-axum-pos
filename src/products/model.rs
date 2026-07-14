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
    /// calculations. Not the selling `price` customers see. Unlike the
    /// rest of this struct, `cost_price` is NOT exposed to every role in
    /// responses: only Owner/Admin get to see it (see `ProductResponse`),
    /// same tier of access as the `/tenants/me/reports/profit` endpoint,
    /// so a Cashier can view a product's catalog data but never its margin.
    pub cost_price: f64,
    pub stock: i32,
    pub created_by: Actor,
}

/// Wire representation of a `Product` returned to clients. Identical to
/// `Product` except `cost_price` is optional and only populated for
/// Owner/Admin — `ProductResponse::from` for any other role omits the
/// field entirely from the JSON (via `skip_serializing_if`) rather than
/// sending `null`, so a Cashier's response looks like the field was never
/// there instead of visibly redacted.
#[derive(Debug, Clone, Serialize)]
pub struct ProductResponse {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub sku: String,
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_price: Option<f64>,
    pub stock: i32,
    pub created_by: Actor,
}

impl ProductResponse {
    pub fn from_product(product: Product, include_cost_price: bool) -> Self {
        Self {
            id: product.id,
            tenant_id: product.tenant_id,
            name: product.name,
            sku: product.sku,
            price: product.price,
            cost_price: include_cost_price.then_some(product.cost_price),
            stock: product.stock,
            created_by: product.created_by,
        }
    }
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
