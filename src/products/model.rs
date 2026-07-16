use serde::{Deserialize, Serialize};

use crate::users::Actor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub tenant_id: String,
    pub name: String,
    pub sku: String,
    pub price: i64,
    /// Purchase price (cost of goods) — the basis for profit report
    /// calculations. Not the selling `price` customers see. Unlike the
    /// rest of this struct, `cost_price` is NOT exposed to every role in
    /// responses: only Owner/Admin get to see it (see `ProductResponse`),
    /// same tier of access as the `/tenants/me/reports/profit` endpoint,
    /// so a Cashier can view a product's catalog data but never its margin.
    pub cost_price: i64,
    pub stock: i32,
    /// Free-form grouping label (e.g. "Beverages", "Snacks") used to filter
    /// `GET /products?category=`. Not a separate `Category` entity with
    /// its own id/table — a plain string is enough for filtering/display
    /// and avoids a whole extra CRUD surface for something that's really
    /// just a tag. Defaults to `"Uncategorized"` (see
    /// `CreateProductRequest`) so every product always has one.
    pub category: String,
    /// Threshold at/below which this product shows up in
    /// `GET /products/low-stock`. Per-product (not a single tenant-wide
    /// number) because a slow-moving product's "low" and a fast-moving
    /// product's "low" are different quantities. Defaults to `5`.
    pub low_stock_threshold: i32,
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
    pub price: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_price: Option<i64>,
    pub stock: i32,
    pub category: String,
    pub low_stock_threshold: i32,
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
            category: product.category,
            low_stock_threshold: product.low_stock_threshold,
            created_by: product.created_by,
        }
    }
}

/// Falls back to `"Uncategorized"` when the client doesn't send a
/// category, so filtering/grouping still has something sensible to show
/// rather than an empty string.
pub const DEFAULT_CATEGORY: &str = "Uncategorized";
/// Falls back to 5 units when the client doesn't set a per-product
/// low-stock threshold.
pub const DEFAULT_LOW_STOCK_THRESHOLD: i32 = 5;

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub sku: String,
    pub price: i64,
    pub cost_price: i64,
    pub stock: i32,
    pub category: Option<String>,
    pub low_stock_threshold: Option<i32>,
}

/// Partial update (all fields optional). `sku` is intentionally NOT
/// changeable through here — sku is treated as a fixed identifier once a
/// product is created, so historical orders that store the sku as a
/// snapshot don't become ambiguous.
#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub price: Option<i64>,
    pub cost_price: Option<i64>,
    pub stock: Option<i32>,
    pub category: Option<String>,
    pub low_stock_threshold: Option<i32>,
}
