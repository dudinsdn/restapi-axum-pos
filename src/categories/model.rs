use serde::{Deserialize, Serialize};

use crate::users::Actor;

/// A product grouping label as its own manageable resource (rather than
/// just the free-form `Product::category` string introduced earlier) —
/// this is what lets Owner/Admin curate a fixed list of categories
/// (rename one, retire one) instead of every typo becoming a new de facto
/// category. `Product::category` is intentionally left as a plain string,
/// NOT a foreign key to this table's `id` — see `CategoryRepository` and
/// `service::list_products_in_category` for why.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub tenant_id: String,
    /// Unique per tenant — see `CategoryRepository::create`. Matched
    /// case-insensitively against `Product::category` when looking up
    /// which products belong to a category (see
    /// `service::list_products_in_category`), since `GET
    /// /products?category=` already does the same.
    pub name: String,
    pub created_by: Actor,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
}

/// Partial update (all fields optional, even though there's currently
/// only one — kept consistent with every other `Update*Request` in this
/// codebase rather than a bare `String` field, so adding a second field
/// later doesn't change the shape of existing requests).
#[derive(Debug, Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
}
