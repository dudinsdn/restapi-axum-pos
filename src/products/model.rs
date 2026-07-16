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
    /// Reference to a `Category` registered via `/categories` — `None`
    /// means "uncategorized" (see `DEFAULT_CATEGORY`). Unlike
    /// `Product::category` below, this is the actual foreign key: setting
    /// it (via `category_id` in `CreateProductRequest`/
    /// `UpdateProductRequest`) is validated against `CategoryRepository`
    /// at write time, the same way `Order::customer_id` is validated
    /// against `CustomerRepository`.
    pub category_id: Option<String>,
    /// Snapshot of the referenced category's name at the time it was set
    /// — same reasoning as `Order::customer_name`: if the category is
    /// later renamed, a product's display shouldn't silently change
    /// out from under an in-progress view, and if the category is
    /// deleted, the product still shows what it used to be (see
    /// `categories::service::delete_category`, which clears
    /// `category_id` back to `None` on affected products but leaves this
    /// string untouched). `"Uncategorized"` when `category_id` is `None`.
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
    pub category_id: Option<String>,
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
            category_id: product.category_id,
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
    /// Id of an already-registered category (see `POST /categories`).
    /// `None` leaves the product uncategorized — there's no free-text
    /// fallback anymore, so a category has to actually exist to be used.
    pub category_id: Option<String>,
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
    /// Same validation as `CreateProductRequest::category_id`. There's
    /// intentionally no way to clear a product back to uncategorized
    /// through here (only to set/change it) — same limitation as
    /// `Customer`'s `email`/`address` fields in this codebase, which can
    /// be set but not explicitly cleared via update either.
    pub category_id: Option<String>,
    pub low_stock_threshold: Option<i32>,
}
